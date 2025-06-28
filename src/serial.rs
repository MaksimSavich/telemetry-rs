use prost::Message;
use serialport::{SerialPort, SerialPortType};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::proto::{Packet, PacketType, Transmission};

// Define the start and end delimiters to match the LoRa module
const START_DELIMITER: &[u8] = b"<START>"; // Start delimiter
const END_DELIMITER: &[u8] = b"<END>"; // End delimiter

// RFD 900x2 configuration
const RFD_BAUD_RATE: u32 = 57600;
const RFD_SCAN_INTERVAL_MS: u64 = 5000; // Scan every 5 seconds

// LoRa configuration
const LORA_BAUD_RATE: u32 = 115200;
const LORA_SCAN_INTERVAL_MS: u64 = 5000; // Scan every 5 seconds

// Connection monitoring - improved intervals
const CONNECTION_CHECK_INTERVAL_MS: u64 = 10000; // Check connection every 10 seconds (less frequent)
const TRANSMISSION_TIMEOUT_MS: u64 = 1000; // Timeout for transmission operations
const CONNECTION_GRACE_PERIOD_MS: u64 = 30000; // Consider disconnected after 30 seconds of no successful transmission

#[derive(Debug, Clone, PartialEq)]
pub enum ModemType {
    Lora,
    Rfd900x,
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
    lora_connection: Arc<Mutex<ModemConnection>>,
    rfd_connection: Arc<Mutex<ModemConnection>>,
    pub lora_status: Arc<Mutex<ModemStatus>>,
    pub rfd_status: Arc<Mutex<ModemStatus>>,
    scan_thread: Option<JoinHandle<()>>,
    scan_running: Arc<Mutex<bool>>,
    lora_enabled: Arc<Mutex<bool>>,
    rfd_enabled: Arc<Mutex<bool>>,
}

impl SerialManager {
    pub fn new() -> Self {
        let lora_connection = Arc::new(Mutex::new(ModemConnection {
            port: None,
            modem_type: ModemType::Lora,
            last_health_check: Instant::now(),
        }));

        let rfd_connection = Arc::new(Mutex::new(ModemConnection {
            port: None,
            modem_type: ModemType::Rfd900x,
            last_health_check: Instant::now(),
        }));

        let instance = Self {
            lora_connection,
            rfd_connection,
            lora_status: Arc::new(Mutex::new(ModemStatus::new())),
            rfd_status: Arc::new(Mutex::new(ModemStatus::new())),
            scan_thread: None,
            scan_running: Arc::new(Mutex::new(false)),
            lora_enabled: Arc::new(Mutex::new(false)), // Default to disabled
            rfd_enabled: Arc::new(Mutex::new(true)),
        };

        instance
    }

    // Enable/disable modems
    pub fn set_lora_enabled(&self, enabled: bool) {
        *self.lora_enabled.lock().unwrap() = enabled;
        if !enabled {
            // Disconnect if disabled
            let mut conn = self.lora_connection.lock().unwrap();
            let mut status = self.lora_status.lock().unwrap();
            conn.port = None;
            status.connected = false;
            status.port_name = None;
            println!("LoRa modem disabled");
        } else {
            println!("LoRa modem enabled");
        }
    }

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

    pub fn is_lora_enabled(&self) -> bool {
        *self.lora_enabled.lock().unwrap()
    }

    pub fn is_rfd_enabled(&self) -> bool {
        *self.rfd_enabled.lock().unwrap()
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
        let lora_connection = Arc::clone(&self.lora_connection);
        let rfd_connection = Arc::clone(&self.rfd_connection);
        let lora_status = Arc::clone(&self.lora_status);
        let rfd_status = Arc::clone(&self.rfd_status);
        let scan_running = Arc::clone(&self.scan_running);
        let lora_enabled = Arc::clone(&self.lora_enabled);
        let rfd_enabled = Arc::clone(&self.rfd_enabled);

        // Spawn a thread to perform scanning
        let scan_thread = thread::spawn(move || {
            let mut last_lora_scan = Instant::now()
                .checked_sub(Duration::from_millis(LORA_SCAN_INTERVAL_MS))
                .unwrap_or_else(Instant::now);
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
                    // Check LoRa connection health passively
                    if *lora_enabled.lock().unwrap() {
                        let mut status = lora_status.lock().unwrap();
                        if status.connected {
                            // Check if we've had recent transmission failures or it's been too long since last success
                            let should_disconnect = if let Some(last_success) = status.last_success
                            {
                                now.duration_since(last_success).as_millis()
                                    > CONNECTION_GRACE_PERIOD_MS as u128
                            } else {
                                true // Never had a successful transmission
                            };

                            if should_disconnect || status.consecutive_failures > 10 {
                                status.connected = false;
                                status.error_message =
                                    Some("Connection health check failed".to_string());
                                let mut conn = lora_connection.lock().unwrap();
                                conn.port = None;
                                println!("LoRa connection marked as unhealthy, will reconnect");
                            }
                        }
                    }

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

                // Scan for LoRa devices if not connected and it's time to scan
                if *lora_enabled.lock().unwrap()
                    && !lora_status.lock().unwrap().connected
                    && now.duration_since(last_lora_scan).as_millis()
                        >= LORA_SCAN_INTERVAL_MS as u128
                {
                    Self::scan_for_modem(
                        &lora_connection,
                        &lora_status,
                        LORA_BAUD_RATE,
                        ModemType::Lora,
                        &Self::verify_lora_connection,
                    );
                    last_lora_scan = now;
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

    // Verify a LoRa connection by sending a settings request
    fn verify_lora_connection(port: &mut Box<dyn SerialPort>) -> Result<(), String> {
        // Create a settings request packet
        let request = Packet {
            r#type: PacketType::Request as i32,
            request: Some(crate::proto::Request {
                settings: true,
                search: false,
                gps: false,
                state_change: 0,
            }),
            settings: None,
            transmission: None,
            log: None,
            gps: None,
            ack: false,
        };

        // Encode and send the packet
        let mut encoded = Vec::new();
        if let Err(e) = request.encode(&mut encoded) {
            return Err(format!("Failed to encode settings request: {}", e));
        }

        // Frame the message with delimiters
        let mut framed_data =
            Vec::with_capacity(START_DELIMITER.len() + encoded.len() + END_DELIMITER.len());
        framed_data.extend_from_slice(START_DELIMITER);
        framed_data.extend_from_slice(&encoded);
        framed_data.extend_from_slice(END_DELIMITER);

        // Try to write to the port
        if let Err(e) = port.write_all(&framed_data) {
            return Err(format!("Failed to write to port: {}", e));
        }

        // Flush the port
        if let Err(e) = port.flush() {
            return Err(format!("Failed to flush port: {}", e));
        }

        // Wait for a response (timeout after 2 seconds)
        let mut buf = [0u8; 1024];
        port.set_timeout(Duration::from_millis(2000))
            .map_err(|e| format!("Failed to set timeout: {}", e))?;

        // Simple check: see if we get any response in the timeout period
        match port.read(&mut buf) {
            Ok(n) if n > 0 => {
                // Got some data, assume it's a valid LoRa device
                println!("LoRa verification successful, received {} bytes", n);
                Ok(())
            }
            Ok(_) => Err("No data received from device".to_string()),
            Err(e) => Err(format!("Failed to read from device: {}", e)),
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

    // Send CAN frame to all enabled and connected modems - OPTIMIZED
    pub fn send_can_frame(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        let lora_connected = self.lora_status.lock().unwrap().connected;
        let rfd_connected = self.rfd_status.lock().unwrap().connected;
        let lora_enabled = self.is_lora_enabled();
        let rfd_enabled = self.is_rfd_enabled();

        let mut errors = Vec::new();
        let mut success_count = 0;

        if lora_enabled && lora_connected {
            match self.send_can_frame_lora_fast(can_id, data) {
                Ok(_) => success_count += 1,
                Err(e) => errors.push(format!("LoRa error: {}", e)),
            }
        }

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

    // Fast LoRa transmission with immediate timeout and status tracking
    fn send_can_frame_lora_fast(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        // Create combined payload with ID (4 bytes) + data
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.extend_from_slice(&can_id.to_be_bytes());
        payload.extend_from_slice(data);

        // Create proto message
        let transmission = Transmission { payload };
        let packet = Packet {
            r#type: PacketType::Transmission as i32,
            transmission: Some(transmission),
            settings: None,
            log: None,
            request: None,
            gps: None,
            ack: false,
        };

        // Encode packet
        let mut encoded = Vec::new();
        packet
            .encode(&mut encoded)
            .map_err(|e| format!("Error encoding packet: {}", e))?;

        // Frame the message
        let mut framed_data =
            Vec::with_capacity(START_DELIMITER.len() + encoded.len() + END_DELIMITER.len());
        framed_data.extend_from_slice(START_DELIMITER);
        framed_data.extend_from_slice(&encoded);
        framed_data.extend_from_slice(END_DELIMITER);

        // Use try_lock to avoid blocking
        let mut lora_conn = match self.lora_connection.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Port is busy - update status but don't fail
                self.update_transmission_status(&self.lora_status, false);
                return Ok(()); // Return Ok to prevent error flooding
            }
        };

        if let Some(port) = lora_conn.port.as_mut() {
            // Set a very short timeout for transmission
            let _ = port.set_timeout(Duration::from_millis(TRANSMISSION_TIMEOUT_MS));

            match port.write_all(&framed_data) {
                Ok(_) => {
                    let _ = port.flush(); // Try to flush but don't fail if it doesn't work
                    self.update_transmission_status(&self.lora_status, true);
                    Ok(())
                }
                Err(e) => {
                    self.update_transmission_status(&self.lora_status, false);
                    Err(format!("Failed to write to LoRa port: {}", e))
                }
            }
        } else {
            self.update_transmission_status(&self.lora_status, false);
            Err("LoRa port not open".to_string())
        }
    }

    // Fast RFD transmission with immediate timeout and status tracking
    fn send_can_frame_rfd_fast(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        // For RFD 900x2, we send the raw CAN ID followed by the data
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.extend_from_slice(&can_id.to_be_bytes());
        payload.extend_from_slice(data);

        println!("CAN ID: {:08X}, Data: {:?}", can_id, payload);

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
            // Set a very short timeout for transmission
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
            lora_connection: Arc::clone(&self.lora_connection),
            rfd_connection: Arc::clone(&self.rfd_connection),
            lora_status: Arc::clone(&self.lora_status),
            rfd_status: Arc::clone(&self.rfd_status),
            scan_thread: None, // Don't clone the thread
            scan_running: Arc::clone(&self.scan_running),
            lora_enabled: Arc::clone(&self.lora_enabled),
            rfd_enabled: Arc::clone(&self.rfd_enabled),
        }
    }
}
