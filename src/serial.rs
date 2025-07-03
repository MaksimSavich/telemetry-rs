// prost import removed - no longer using protobuf
use serialport::{SerialPort, SerialPortType};
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use crc32fast::Hasher;

// Proto imports removed - no longer using LoRa protocol

// Robust framing protocol - no external dependencies, handles edge cases
const FRAME_START: &[u8] = b"\xAA\xBB\xCC\xDD"; // 4-byte start marker
const FRAME_END: &[u8] = b"\xEE\xFF\x00\x11";   // 4-byte end marker
const ESCAPE_BYTE: u8 = 0x7D;                    // Escape byte for data
const ESCAPE_XOR: u8 = 0x20;                     // XOR value for escaping

// Enhanced batching configuration - conservative settings for reliability
const MAX_BATCH_SIZE: usize = 4; // Much smaller batches for serial reliability
const MAX_BATCH_BYTES: usize = 60; // Very conservative byte limit
const BATCH_TIMEOUT_MS: u64 = 50; // Longer timeout for stability
const MIN_BATCH_SIZE: usize = 1; // Always send at least 1 frame

// Radio configuration constants
const RFD_BAUD_RATE: u32 = 57600; // RFD modem standard baud rate
const CONNECTION_CHECK_INTERVAL_MS: u64 = 10000; // Slower connection checks
const TRANSMISSION_TIMEOUT_MS: u64 = 200; // Much longer timeout for reliability
const CONNECTION_GRACE_PERIOD_MS: u64 = 30000;
const RFD_SCAN_INTERVAL_MS: u64 = 5000;

#[derive(Debug, Clone, PartialEq)]
pub enum ModemType {
    Rfd900x,
}

#[derive(Debug, Clone)]
pub struct CanFrameData {
    pub id: u32,
    pub data: Vec<u8>,
    pub timestamp: Instant,
    pub sequence_number: u64,
    pub priority: MessagePriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    Critical = 0,  // Safety-critical messages (faults, emergency stops)
    High = 1,      // Operational messages (motor control, BMS power)
    Medium = 2,    // Status messages (temperatures, states)
    Low = 3,       // Monitoring messages (limits, capacity)
}

static SEQUENCE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

// Robust framing utilities
fn escape_data(data: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::with_capacity(data.len() * 2); // Pre-allocate for worst case
    
    for &byte in data {
        if byte == FRAME_START[0] || byte == FRAME_START[1] || byte == FRAME_START[2] || byte == FRAME_START[3] ||
           byte == FRAME_END[0] || byte == FRAME_END[1] || byte == FRAME_END[2] || byte == FRAME_END[3] ||
           byte == ESCAPE_BYTE {
            escaped.push(ESCAPE_BYTE);
            escaped.push(byte ^ ESCAPE_XOR);
        } else {
            escaped.push(byte);
        }
    }
    
    escaped
}

fn unescape_data(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let mut unescaped = Vec::with_capacity(data.len());
    let mut i = 0;
    
    while i < data.len() {
        if data[i] == ESCAPE_BYTE {
            if i + 1 >= data.len() {
                return Err("Incomplete escape sequence");
            }
            unescaped.push(data[i + 1] ^ ESCAPE_XOR);
            i += 2;
        } else {
            unescaped.push(data[i]);
            i += 1;
        }
    }
    
    Ok(unescaped)
}

impl CanFrameData {
    pub fn new(id: u32, data: &[u8]) -> Self {
        let sequence_number = SEQUENCE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            id,
            data: data.to_vec(),
            timestamp: Instant::now(),
            sequence_number,
            priority: Self::get_priority_for_id(id),
        }
    }

    fn get_priority_for_id(id: u32) -> MessagePriority {
        match id {
            // Critical safety messages
            0x300 => MessagePriority::Critical,  // BMS DTC flags
            0x776 | 0x777 => MessagePriority::Critical,  // BPS System status
            
            // High priority operational messages
            0x320 => MessagePriority::High,      // BMS Power data
            0x0CF11E05 | 0x0CF11F05 => MessagePriority::High,  // Motor controller data
            0x0CF11E06 | 0x0CF11F06 => MessagePriority::High,  // Motor controller data
            
            // Medium priority status messages
            0x360 => MessagePriority::Medium,    // BMS Temperature
            0x330 => MessagePriority::Medium,    // BMS State
            
            // Low priority monitoring messages
            0x310 => MessagePriority::Low,       // BMS Limits
            0x340 => MessagePriority::Low,       // BMS Capacity
            0x200..=0x203 => MessagePriority::Low,  // MPPT data
            
            // Default to medium priority
            _ => MessagePriority::Medium,
        }
    }

    // Enhanced serialization with CRC32 validation and sequence number
    pub fn to_bytes(&self) -> Vec<u8> {
        // Validate data length (CAN max is 8 bytes)
        let data_len = std::cmp::min(self.data.len(), 8);

        let mut bytes = Vec::with_capacity(17 + data_len);
        bytes.extend_from_slice(&self.id.to_be_bytes()); // 4 bytes ID (big-endian)
        bytes.push(data_len as u8); // 1 byte length
        bytes.extend_from_slice(&self.data[..data_len]); // data (validated length)
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes()); // 8 bytes sequence number
        
        // Calculate CRC32 of the payload
        let mut hasher = Hasher::new();
        hasher.update(&bytes);
        let crc = hasher.finalize();
        bytes.extend_from_slice(&crc.to_be_bytes()); // 4 bytes CRC32
        
        bytes
    }

    // Deserialize from bytes with CRC32 validation and sequence number
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 17 {
            return None;
        }

        let id = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let data_len = bytes[4] as usize;

        if bytes.len() < 17 + data_len || data_len > 8 {
            return None;
        }

        let data = bytes[5..5 + data_len].to_vec();
        let sequence_number = u64::from_be_bytes([
            bytes[5 + data_len],
            bytes[6 + data_len],
            bytes[7 + data_len],
            bytes[8 + data_len],
            bytes[9 + data_len],
            bytes[10 + data_len],
            bytes[11 + data_len],
            bytes[12 + data_len],
        ]);
        
        // Verify CRC32
        let expected_crc = u32::from_be_bytes([
            bytes[13 + data_len],
            bytes[14 + data_len],
            bytes[15 + data_len],
            bytes[16 + data_len],
        ]);
        
        let mut hasher = Hasher::new();
        hasher.update(&bytes[..13 + data_len]);
        let actual_crc = hasher.finalize();
        
        if actual_crc != expected_crc {
            println!("CRC32 validation failed for CAN ID 0x{:X}: expected 0x{:X}, got 0x{:X}", 
                     id, expected_crc, actual_crc);
            return None;
        }

        Some(Self {
            id,
            data,
            timestamp: Instant::now(),
            sequence_number,
            priority: Self::get_priority_for_id(id),
        })
    }
}

// Enhanced frame filtering to reduce message spam
pub struct FrameFilter {
    last_transmission: HashMap<u32, Instant>,
    min_intervals: HashMap<u32, Duration>,
}

impl FrameFilter {
    pub fn new() -> Self {
        let mut filter = Self {
            last_transmission: HashMap::new(),
            min_intervals: HashMap::new(),
        };

        // Conservative transmission intervals for reliability (reverted)
        filter
            .min_intervals
            .insert(0x300, Duration::from_millis(100)); // DTC flags - max 10 Hz
        filter
            .min_intervals
            .insert(0x320, Duration::from_millis(50)); // BMS Power - max 20 Hz
        filter
            .min_intervals
            .insert(0x360, Duration::from_millis(50)); // BMS Temp - max 20 Hz
        filter
            .min_intervals
            .insert(0x310, Duration::from_millis(2000)); // BMS Limits - max 0.5 Hz
        filter
            .min_intervals
            .insert(0x330, Duration::from_millis(1000)); // BMS State - max 1 Hz
        filter
            .min_intervals
            .insert(0x340, Duration::from_millis(1000)); // BMS Capacity - max 1 Hz

        // Motor controllers - respect their 50ms intervals from DBC
        filter
            .min_intervals
            .insert(0x0CF11E05, Duration::from_millis(50));
        filter
            .min_intervals
            .insert(0x0CF11F05, Duration::from_millis(50));
        filter
            .min_intervals
            .insert(0x0CF11E06, Duration::from_millis(50));
        filter
            .min_intervals
            .insert(0x0CF11F06, Duration::from_millis(50));

        // MPPT - 500ms and 1000ms from DBC
        filter
            .min_intervals
            .insert(0x200, Duration::from_millis(500));
        filter
            .min_intervals
            .insert(0x201, Duration::from_millis(1000));
        filter
            .min_intervals
            .insert(0x202, Duration::from_millis(500));
        filter
            .min_intervals
            .insert(0x203, Duration::from_millis(1000));

        // BPS - 100ms from DBC
        filter
            .min_intervals
            .insert(0x776, Duration::from_millis(100));
        filter
            .min_intervals
            .insert(0x777, Duration::from_millis(100));

        filter
    }

    pub fn should_transmit(&mut self, frame: &CanFrameData) -> bool {
        let now = Instant::now();
        let can_id = frame.id;

        // Check if we have a minimum interval for this message
        if let Some(min_interval) = self.min_intervals.get(&can_id) {
            if let Some(last_time) = self.last_transmission.get(&can_id) {
                if now.duration_since(*last_time) < *min_interval {
                    return false; // Too soon, filter out
                }
            }
        } else {
            // Default minimum interval for unknown messages
            let default_interval = Duration::from_millis(10);
            
            if let Some(last_time) = self.last_transmission.get(&can_id) {
                if now.duration_since(*last_time) < default_interval {
                    return false;
                }
            }
        }

        // Update last transmission time
        self.last_transmission.insert(can_id, now);
        true
    }
}

pub struct ImprovedFrameBatcher {
    // Priority-based latest message storage: CAN ID -> (Frame, Priority)
    latest_frames: HashMap<u32, CanFrameData>,
    // Maintain insertion order for same-priority messages
    frame_order: VecDeque<u32>,
    last_send: Instant,
    total_bytes: usize,
    frame_filter: FrameFilter,
    batch_count: u64,
    // Statistics
    total_frames_added: u64,
    frames_replaced: u64,
}

impl ImprovedFrameBatcher {
    pub fn new() -> Self {
        Self {
            latest_frames: HashMap::new(),
            frame_order: VecDeque::new(),
            last_send: Instant::now(),
            total_bytes: 0,
            frame_filter: FrameFilter::new(),
            batch_count: 0,
            total_frames_added: 0,
            frames_replaced: 0,
        }
    }

    pub fn add_frame(&mut self, frame: CanFrameData) -> bool {
        // Apply filtering to reduce spam
        if !self.frame_filter.should_transmit(&frame) {
            return true; // Frame filtered out, but don't report as error
        }

        let frame_size = 13 + std::cmp::min(frame.data.len(), 8); // Updated size with sequence number
        let can_id = frame.id;
        
        self.total_frames_added += 1;
        
        // Check if we already have this message ID
        if let Some(existing_frame) = self.latest_frames.get(&can_id) {
            let old_seq = existing_frame.sequence_number;
            let new_seq = frame.sequence_number;
            // Replace with newer message (higher sequence number or more recent timestamp)
            if new_seq > old_seq || 
               (new_seq == old_seq && frame.timestamp > existing_frame.timestamp) {
                self.latest_frames.insert(can_id, frame);
                self.frames_replaced += 1;
                println!("Replaced message 0x{:X} with newer version (seq: {} -> {})", 
                         can_id, old_seq, new_seq);
            }
            return true; // Always succeed when replacing
        }
        
        // Check batch limits for new messages
        if self.latest_frames.len() >= MAX_BATCH_SIZE || self.get_total_bytes() + frame_size > MAX_BATCH_BYTES {
            return false; // Batch is full
        }

        // Add new message
        self.latest_frames.insert(can_id, frame);
        self.frame_order.push_back(can_id);
        true
    }

    pub fn should_send(&self) -> bool {
        if self.latest_frames.is_empty() {
            return false;
        }

        // Send conditions
        self.latest_frames.len() >= MAX_BATCH_SIZE
            || self.get_total_bytes() >= MAX_BATCH_BYTES
            || (self.latest_frames.len() >= MIN_BATCH_SIZE
                && self.last_send.elapsed().as_millis() >= BATCH_TIMEOUT_MS as u128)
    }
    
    fn get_total_bytes(&self) -> usize {
        self.latest_frames.values()
            .map(|frame| 17 + std::cmp::min(frame.data.len(), 8))  // Updated for CRC32
            .sum()
    }

    pub fn create_batch(&mut self) -> Vec<u8> {
        if self.latest_frames.is_empty() {
            return Vec::new();
        }

        let mut payload = Vec::new();

        // Collect frames and sort by priority (critical first, then by timestamp)
        let mut frames_to_send: Vec<_> = self.latest_frames.values().cloned().collect();
        frames_to_send.sort_by(|a, b| {
            // First sort by priority (critical messages first)
            match a.priority.cmp(&b.priority) {
                std::cmp::Ordering::Equal => {
                    // Within same priority, prefer more recent messages
                    b.timestamp.cmp(&a.timestamp)
                }
                other => other,
            }
        });

        // Limit to batch size
        let frame_count = std::cmp::min(frames_to_send.len(), MAX_BATCH_SIZE);
        payload.extend_from_slice(&(frame_count as u16).to_be_bytes());

        // Add frames (priority-ordered)
        let mut actual_count = 0;
        for frame in frames_to_send.iter().take(frame_count) {
            let frame_bytes = frame.to_bytes();
            payload.extend_from_slice(&frame_bytes);
            actual_count += 1;
        }

        // Update frame count if different
        if actual_count != frame_count {
            let count_bytes = &(actual_count as u16).to_be_bytes();
            payload[0] = count_bytes[0];
            payload[1] = count_bytes[1];
        }

        // Calculate CRC32 for the entire payload
        let mut hasher = Hasher::new();
        hasher.update(&payload);
        let crc = hasher.finalize();
        payload.extend_from_slice(&crc.to_be_bytes());

        // Frame the payload with start/end markers and escaping
        let mut framed = Vec::new();
        
        // Add start marker
        framed.extend_from_slice(FRAME_START);
        
        // Escape and add the payload
        let escaped_payload = escape_data(&payload);
        framed.extend_from_slice(&escaped_payload);
        
        // Add end marker
        framed.extend_from_slice(FRAME_END);

        // Clear sent frames
        self.latest_frames.clear();
        self.frame_order.clear();
        self.total_bytes = 0;
        self.last_send = Instant::now();
        self.batch_count += 1;

        println!(
            "Created framed batch #{}: {} frames, {} bytes total (replaced: {})",
            self.batch_count,
            actual_count,
            framed.len(),
            self.frames_replaced
        );
        
        // Reset replacement counter
        self.frames_replaced = 0;
        
        framed
    }

    pub fn is_empty(&self) -> bool {
        self.latest_frames.is_empty()
    }

    pub fn get_queue_size(&self) -> usize {
        self.latest_frames.len()
    }
    
    pub fn get_stats(&self) -> (u64, u64, usize) {
        (self.total_frames_added, self.frames_replaced, self.latest_frames.len())
    }
}

#[derive(Debug, Clone)]
pub struct ModemStatus {
    pub connected: bool,
    pub port_name: Option<String>,
    pub last_success: Option<Instant>,
    pub error_message: Option<String>,
    pub last_transmission_attempt: Option<Instant>,
    pub consecutive_failures: u32,
}

impl ModemStatus {
    fn new() -> Self {
        Self {
            connected: false,
            port_name: None,
            last_success: None,
            error_message: None,
            last_transmission_attempt: None,
            consecutive_failures: 0,
        }
    }
}

struct ModemConnection {
    port: Option<Box<dyn SerialPort>>,
    modem_type: ModemType,
    last_health_check: Instant,
}

pub struct SerialManager {
    rfd_connection: Arc<Mutex<ModemConnection>>,
    pub rfd_status: Arc<Mutex<ModemStatus>>,
    scan_thread: Option<JoinHandle<()>>,
    scan_running: Arc<Mutex<bool>>,
    rfd_enabled: Arc<Mutex<bool>>,

    // Enhanced batching fields
    rfd_batcher: Arc<Mutex<ImprovedFrameBatcher>>,
    batch_thread: Option<JoinHandle<()>>,
    batching_enabled: Arc<Mutex<bool>>,
}

impl SerialManager {
    pub fn new() -> Self {
        let rfd_connection = Arc::new(Mutex::new(ModemConnection {
            port: None,
            modem_type: ModemType::Rfd900x,
            last_health_check: Instant::now(),
        }));

        Self {
            rfd_connection,
            rfd_status: Arc::new(Mutex::new(ModemStatus::new())),
            scan_thread: None,
            scan_running: Arc::new(Mutex::new(false)),
            rfd_enabled: Arc::new(Mutex::new(true)),
            rfd_batcher: Arc::new(Mutex::new(ImprovedFrameBatcher::new())),
            batch_thread: None,
            batching_enabled: Arc::new(Mutex::new(true)),
        }
    }

    // Enable/disable modems
    pub fn set_rfd_enabled(&self, enabled: bool) {
        *self.rfd_enabled.lock().unwrap() = enabled;
        if !enabled {
            // Disconnect if disabled
            let mut conn = self.rfd_connection.lock().unwrap();
            let mut status = self.rfd_status.lock().unwrap();
            conn.port = None;
            status.connected = false;
            status.port_name = None;
            println!("RFD 900x2 modem disabled");
        } else {
            println!("RFD 900x2 modem enabled");
        }
    }

    pub fn is_rfd_enabled(&self) -> bool {
        *self.rfd_enabled.lock().unwrap()
    }

    // Optimized CAN frame sending with enhanced batching
    pub fn send_can_frame(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        let frame = CanFrameData::new(can_id, data);
        let rfd_enabled = self.is_rfd_enabled();
        let batching_enabled = *self.batching_enabled.lock().unwrap();

        if !batching_enabled {
            // Fall back to individual transmission
            return self.send_can_frame_individual(can_id, data);
        }

        let mut success_count = 0;
        let mut errors = Vec::new();

        // Add to RFD batch
        if rfd_enabled && self.rfd_status.lock().unwrap().connected {
            let mut batcher = self.rfd_batcher.lock().unwrap();
            if !batcher.add_frame(frame.clone()) {
                // Batch is full, force send current batch and retry
                drop(batcher);
                self.force_send_rfd_batch();
                let mut batcher = self.rfd_batcher.lock().unwrap();
                if batcher.add_frame(frame) {
                    success_count += 1;
                } else {
                    errors.push("RFD batch overflow".to_string());
                }
            } else {
                success_count += 1;
            }
        }

        if success_count > 0 {
            Ok(())
        } else if !errors.is_empty() {
            Err(errors.join("; "))
        } else {
            Err("No modems available for batching".to_string())
        }
    }

    fn force_send_rfd_batch(&self) {
        let batch_data = {
            let mut batcher = self.rfd_batcher.lock().unwrap();
            if batcher.is_empty() {
                return;
            }
            batcher.create_batch()
        };

        Self::send_rfd_batch_improved(&self.rfd_connection, &self.rfd_status, &batch_data);
    }

    // Enhanced RFD batch sending with proper framing and error handling
    fn send_rfd_batch_improved(
        connection: &Arc<Mutex<ModemConnection>>,
        status: &Arc<Mutex<ModemStatus>>,
        batch_data: &[u8],
    ) {
        if batch_data.is_empty() {
            return;
        }

        let mut conn = match connection.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                println!("RFD port busy, skipping batch");
                return;
            }
        };

        if let Some(port) = conn.port.as_mut() {
            // Set timeout for transmission
            let _ = port.set_timeout(Duration::from_millis(TRANSMISSION_TIMEOUT_MS));

            // Send batch as-is (already has markers and checksum)
            match port.write_all(batch_data) {
                Ok(_) => {
                    if let Err(e) = port.flush() {
                        println!("RFD flush error: {}", e);
                        Self::update_transmission_status_static(status, false);
                    } else {
                        // Uncomment for detailed logging:
                        // println!("RFD batch sent: {} bytes", batch_data.len());
                        Self::update_transmission_status_static(status, true);
                    }
                }
                Err(e) => {
                    println!("RFD write error: {}", e);
                    Self::update_transmission_status_static(status, false);
                }
            }
        } else {
            println!("RFD port not available");
            Self::update_transmission_status_static(status, false);
        }
    }

    fn update_transmission_status_static(status_arc: &Arc<Mutex<ModemStatus>>, success: bool) {
        if let Ok(mut status) = status_arc.try_lock() {
            status.last_transmission_attempt = Some(Instant::now());

            if success {
                status.last_success = Some(Instant::now());
                status.consecutive_failures = 0;
            } else {
                status.consecutive_failures += 1;
            }
        }
    }

    // Fallback individual transmission method (for compatibility)
    fn send_can_frame_individual(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        let rfd_connected = self.rfd_status.lock().unwrap().connected;
        let rfd_enabled = self.is_rfd_enabled();

        let mut errors = Vec::new();
        let mut success_count = 0;

        if rfd_enabled && rfd_connected {
            match self.send_can_frame_rfd_fast(can_id, data) {
                Ok(_) => success_count += 1,
                Err(e) => errors.push(format!("RFD error: {}", e)),
            }
        }

        if success_count > 0 {
            Ok(())
        } else if !errors.is_empty() {
            Err(errors.join("; "))
        } else {
            Err("No modems available for transmission".to_string())
        }
    }

    // Fast RFD transmission (individual frames)
    fn send_can_frame_rfd_fast(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        // For RFD 900x2, we send the raw CAN ID followed by the data
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.extend_from_slice(&can_id.to_be_bytes());
        payload.extend_from_slice(data);

        // Use try_lock to avoid blocking
        let mut rfd_conn = match self.rfd_connection.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Port is busy - update status but don't fail
                self.update_transmission_status(&self.rfd_status, false);
                return Ok(()); // Return Ok to prevent error flooding
            }
        };

        if let Some(port) = rfd_conn.port.as_mut() {
            // Set a short timeout for transmission
            let _ = port.set_timeout(Duration::from_millis(TRANSMISSION_TIMEOUT_MS));

            match port.write_all(&payload) {
                Ok(_) => {
                    let _ = port.flush(); // Try to flush but don't fail if it doesn't work
                    self.update_transmission_status(&self.rfd_status, true);
                    Ok(())
                }
                Err(e) => {
                    self.update_transmission_status(&self.rfd_status, false);
                    Err(format!("Failed to write to RFD port: {}", e))
                }
            }
        } else {
            self.update_transmission_status(&self.rfd_status, false);
            Err("RFD port not open".to_string())
        }
    }

    // Update transmission status for passive health monitoring
    fn update_transmission_status(&self, status_arc: &Arc<Mutex<ModemStatus>>, success: bool) {
        if let Ok(mut status) = status_arc.try_lock() {
            status.last_transmission_attempt = Some(Instant::now());

            if success {
                status.last_success = Some(Instant::now());
                status.consecutive_failures = 0;
            } else {
                status.consecutive_failures += 1;
            }
        }
    }

    pub fn enable_batching(&self, enabled: bool) {
        *self.batching_enabled.lock().unwrap() = enabled;
        if enabled {
            println!("Enhanced batching enabled");
        } else {
            println!("Enhanced batching disabled");
        }
    }

    pub fn get_batch_stats(&self) -> usize {
        self.rfd_batcher.lock().unwrap().get_queue_size()
    }

    // Start scanning for modems in the background
    pub fn start_background_scanning(&mut self) -> Result<(), String> {
        // If a scan is already running, don't start another one
        let mut scan_running = self.scan_running.lock().unwrap();
        if *scan_running {
            return Ok(());
        }
        *scan_running = true;

        // Clone the necessary Arc references for the thread
        let rfd_connection = Arc::clone(&self.rfd_connection);
        let rfd_status = Arc::clone(&self.rfd_status);
        let scan_running = Arc::clone(&self.scan_running);
        let rfd_enabled = Arc::clone(&self.rfd_enabled);

        // Spawn a thread to perform scanning
        let scan_thread = thread::spawn(move || {
            let mut last_rfd_scan = Instant::now()
                .checked_sub(Duration::from_millis(RFD_SCAN_INTERVAL_MS))
                .unwrap_or_else(Instant::now);
            let mut last_connection_check = Instant::now();

            loop {
                // Check if we should stop scanning
                if !*scan_running.lock().unwrap() {
                    break;
                }

                let now = Instant::now();

                // Passive connection health monitoring (based on transmission success/failure)
                if now.duration_since(last_connection_check).as_millis()
                    >= CONNECTION_CHECK_INTERVAL_MS as u128
                {
                    // Check RFD connection health passively
                    if *rfd_enabled.lock().unwrap() {
                        let mut status = rfd_status.lock().unwrap();
                        if status.connected {
                            let should_disconnect = if let Some(last_success) = status.last_success
                            {
                                now.duration_since(last_success).as_millis()
                                    > CONNECTION_GRACE_PERIOD_MS as u128
                            } else {
                                true
                            };

                            if should_disconnect || status.consecutive_failures > 10 {
                                status.connected = false;
                                status.error_message =
                                    Some("Connection health check failed".to_string());
                                let mut conn = rfd_connection.lock().unwrap();
                                conn.port = None;
                                println!(
                                    "RFD 900x2 connection marked as unhealthy, will reconnect"
                                );
                            }
                        }
                    }

                    last_connection_check = now;
                }

                // Scan for RFD devices if not connected and it's time to scan
                if *rfd_enabled.lock().unwrap()
                    && !rfd_status.lock().unwrap().connected
                    && now.duration_since(last_rfd_scan).as_millis() >= RFD_SCAN_INTERVAL_MS as u128
                {
                    Self::scan_for_modem(
                        &rfd_connection,
                        &rfd_status,
                        RFD_BAUD_RATE,
                        ModemType::Rfd900x,
                        &Self::verify_rfd_connection,
                    );
                    last_rfd_scan = now;
                }

                // Sleep to avoid using too much CPU
                thread::sleep(Duration::from_millis(100));
            }
        });

        self.scan_thread = Some(scan_thread);
        Ok(())
    }

    // Stop background scanning
    pub fn stop_background_scanning(&mut self) {
        if let Ok(mut running) = self.scan_running.lock() {
            *running = false;
        }

        if let Some(thread) = self.scan_thread.take() {
            let _ = thread.join();
        }
    }

    // Scan for a specific modem type
    fn scan_for_modem<F>(
        connection: &Arc<Mutex<ModemConnection>>,
        status: &Arc<Mutex<ModemStatus>>,
        baud_rate: u32,
        modem_type: ModemType,
        verify_fn: &F,
    ) where
        F: Fn(&mut Box<dyn SerialPort>) -> Result<(), String>,
    {
        let ports = Self::list_available_ports();

        // Keep track of current port name to avoid reconnecting to the same port
        let current_port_name = status.lock().unwrap().port_name.clone();

        for port_name in ports {
            // Skip if we're already connected to this port
            if let Some(current) = &current_port_name {
                if &port_name == current {
                    continue;
                }
            }

            // Try to open the port
            match serialport::new(&port_name, baud_rate)
                .timeout(Duration::from_millis(1000))
                .open()
            {
                Ok(mut port) => {
                    // Try to verify the connected device
                    match verify_fn(&mut port) {
                        Ok(()) => {
                            // Device verified! Update connection and status
                            let mut conn = connection.lock().unwrap();
                            let mut stat = status.lock().unwrap();

                            conn.port = Some(port);
                            conn.modem_type = modem_type.clone();
                            conn.last_health_check = Instant::now();

                            stat.connected = true;
                            stat.port_name = Some(port_name.clone());
                            stat.last_success = Some(Instant::now());
                            stat.error_message = None;
                            stat.consecutive_failures = 0;

                            println!("{:?} modem connected on port {}", modem_type, port_name);
                            break;
                        }
                        Err(e) => {
                            // Not the right device, continue scanning
                            println!(
                                "Failed to verify {:?} on port {}: {}",
                                modem_type, port_name, e
                            );
                            continue;
                        }
                    }
                }
                Err(e) => {
                    // Couldn't open the port, try next
                    println!("Failed to open port {}: {}", port_name, e);
                    continue;
                }
            }
        }
    }

    // Simplified RFD verification - just check if we can open the port
    fn verify_rfd_connection(port: &mut Box<dyn SerialPort>) -> Result<(), String> {
        // For RFD, we'll use a simpler verification method to avoid interfering with transmission
        // Just try to set the timeout and flush - if this works, assume it's an RFD

        // Set timeout
        port.set_timeout(Duration::from_millis(100))
            .map_err(|e| format!("Failed to set timeout: {}", e))?;

        // Try to flush - this is a simple operation that should work on any serial device
        port.flush()
            .map_err(|e| format!("Failed to flush port: {}", e))?;

        // If we get here, the port is working
        println!("RFD 900x2 verification successful (simple check)");
        Ok(())
    }

    // Enhanced batch thread with better error handling and statistics
    pub fn start_batching(&mut self) -> Result<(), String> {
        if self.batch_thread.is_some() {
            return Ok(()); // Already running
        }

        let rfd_batcher = Arc::clone(&self.rfd_batcher);
        let rfd_connection = Arc::clone(&self.rfd_connection);
        let rfd_status = Arc::clone(&self.rfd_status);
        let batching_enabled = Arc::clone(&self.batching_enabled);
        let rfd_enabled = Arc::clone(&self.rfd_enabled);

        let batch_thread = thread::spawn(move || {
            let mut last_stats = Instant::now();
            let mut rfd_batch_count = 0u64;

            println!("Enhanced batch thread started");

            loop {
                if !*batching_enabled.lock().unwrap() {
                    break;
                }

                let mut sent_batch = false;

                // Check RFD batching
                let rfd_enabled_guard = match rfd_enabled.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        println!("RFD enabled mutex poisoned, skipping batch");
                        continue;
                    }
                };
                let rfd_status_guard = match rfd_status.lock() {
                    Ok(guard) => guard,
                    Err(_) => {
                        println!("RFD status mutex poisoned, skipping batch");
                        continue;
                    }
                };
                
                if *rfd_enabled_guard && rfd_status_guard.connected {
                    drop(rfd_enabled_guard);
                    drop(rfd_status_guard);
                    
                    let should_send = {
                        match rfd_batcher.lock() {
                            Ok(batcher) => batcher.should_send(),
                            Err(_) => {
                                println!("RFD batcher mutex poisoned, skipping batch");
                                continue;
                            }
                        }
                    };

                    if should_send {
                        let batch_data = {
                            match rfd_batcher.lock() {
                                Ok(mut batcher) => batcher.create_batch(),
                                Err(_) => {
                                    println!("RFD batcher mutex poisoned, skipping batch");
                                    continue;
                                }
                            }
                        };

                        if !batch_data.is_empty() {
                            Self::send_rfd_batch_improved(
                                &rfd_connection,
                                &rfd_status,
                                &batch_data,
                            );
                            sent_batch = true;
                            rfd_batch_count += 1;
                        }
                    }
                }

                // Print stats every 10 seconds
                if last_stats.elapsed().as_secs() >= 10 {
                    let rfd_queue = match rfd_batcher.lock() {
                        Ok(batcher) => batcher.get_queue_size(),
                        Err(_) => 0,
                    };

                    println!(
                        "Batch stats (10s): RFD: {} batches ({} queued)",
                        rfd_batch_count, rfd_queue
                    );
                    rfd_batch_count = 0;
                    last_stats = Instant::now();
                }

                // Sleep based on whether we sent a batch
                if sent_batch {
                    thread::sleep(Duration::from_millis(1)); // Quick check after sending
                } else {
                    thread::sleep(Duration::from_millis(5)); // Longer sleep when idle
                }
            }

            println!("Enhanced batch thread stopped");
        });

        self.batch_thread = Some(batch_thread);
        Ok(())
    }

    // List available serial ports
    pub fn list_available_ports() -> Vec<String> {
        match serialport::available_ports() {
            Ok(ports) => {
                ports
                    .iter()
                    .filter_map(|port| {
                        // Filter for USB serial devices if possible
                        match &port.port_type {
                            SerialPortType::UsbPort(_) => Some(port.port_name.clone()),
                            _ => Some(port.port_name.clone()), // Include all ports for now
                        }
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }
}

impl Clone for SerialManager {
    fn clone(&self) -> Self {
        Self {
            rfd_connection: Arc::clone(&self.rfd_connection),
            rfd_status: Arc::clone(&self.rfd_status),
            scan_thread: None, // Don't clone the thread
            scan_running: Arc::clone(&self.scan_running),
            rfd_enabled: Arc::clone(&self.rfd_enabled),
            rfd_batcher: Arc::clone(&self.rfd_batcher),
            batch_thread: None, // Don't clone the thread
            batching_enabled: Arc::clone(&self.batching_enabled),
        }
    }
}

// Utility functions for parsing received framed batches
pub fn parse_can_batch(batch_data: &[u8]) -> Vec<CanFrameData> {
    let mut frames = Vec::new();

    if batch_data.len() < FRAME_START.len() + FRAME_END.len() + 6 {
        return frames; // Too small for valid frame
    }

    // Check for start marker
    if &batch_data[0..FRAME_START.len()] != FRAME_START {
        println!("Invalid start marker in batch");
        return frames;
    }

    // Find end marker
    let end_marker_pos = batch_data.len().saturating_sub(FRAME_END.len());
    if &batch_data[end_marker_pos..] != FRAME_END {
        println!("Invalid end marker in batch");
        return frames;
    }

    // Extract the escaped payload (between markers)
    let escaped_payload = &batch_data[FRAME_START.len()..end_marker_pos];
    
    // Unescape the payload
    let payload = match unescape_data(escaped_payload) {
        Ok(data) => data,
        Err(e) => {
            println!("Unescape failed: {}", e);
            return frames;
        }
    };

    if payload.len() < 6 {
        return frames; // Need at least frame count (2) + CRC (4)
    }

    // Verify CRC32 of the payload (exclude the last 4 CRC bytes)
    let payload_len = payload.len() - 4;
    let mut hasher = Hasher::new();
    hasher.update(&payload[..payload_len]);
    let expected_crc = hasher.finalize();
    
    let actual_crc = u32::from_be_bytes([
        payload[payload_len],
        payload[payload_len + 1],
        payload[payload_len + 2],
        payload[payload_len + 3],
    ]);
    
    if actual_crc != expected_crc {
        println!("Batch CRC32 validation failed: expected 0x{:X}, got 0x{:X}", 
                 expected_crc, actual_crc);
        return frames;
    }

    let frame_count = u16::from_be_bytes([payload[0], payload[1]]) as usize;
    let mut offset = 2; // Skip frame count

    for _ in 0..frame_count {
        if offset >= payload_len {
            break;
        }

        // Find the end of this frame (17 + data_len bytes with CRC32)
        if let Some(frame) = CanFrameData::from_bytes(&payload[offset..payload_len]) {
            let frame_size = 17 + frame.data.len(); // Updated for CRC32 format
            frames.push(frame);
            offset += frame_size;
        } else {
            break; // Invalid frame format
        }
    }

    frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_frame_serialization() {
        let frame = CanFrameData::new(0x123, &[0xAA, 0xBB, 0xCC]);
        let bytes = frame.to_bytes();
        let parsed = CanFrameData::from_bytes(&bytes).unwrap();

        assert_eq!(frame.id, parsed.id);
        assert_eq!(frame.data, parsed.data);
    }

    #[test]
    fn test_enhanced_batching() {
        let mut batcher = ImprovedFrameBatcher::new();

        // Add some frames
        let frame1 = CanFrameData::new(0x100, &[1, 2, 3, 4]);
        let frame2 = CanFrameData::new(0x200, &[5, 6, 7, 8]);

        assert!(batcher.add_frame(frame1));
        assert!(batcher.add_frame(frame2));

        let batch = batcher.create_batch();

        // Verify batch structure
        assert!(batch.len() > 10); // Should have markers, count, frames, checksum
        assert_eq!(&batch[0..4], BATCH_START_MARKER);

        let parsed_frames = parse_can_batch(&batch);
        assert_eq!(parsed_frames.len(), 2);
        assert_eq!(parsed_frames[0].id, 0x100);
        assert_eq!(parsed_frames[1].id, 0x200);
    }

    #[test]
    fn test_frame_filtering() {
        let mut filter = FrameFilter::new();

        // Test rapid messages get filtered
        let frame = CanFrameData::new(0x300, &[0x40, 0x00, 0x04, 0x00]);

        assert!(filter.should_transmit(&frame)); // First message allowed
        assert!(!filter.should_transmit(&frame)); // Second message too soon, filtered

        // Wait for interval to pass (would need to mock time in real test)
        // assert!(filter.should_transmit(&frame)); // After interval, allowed again
    }
}
