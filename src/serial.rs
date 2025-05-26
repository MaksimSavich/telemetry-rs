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

// Connection monitoring
const CONNECTION_CHECK_INTERVAL_MS: u64 = 2000; // Check connection every 2 seconds

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
}

impl ModemStatus {
    fn new() -> Self {
        Self {
            connected: false,
            port_name: None,
            last_success: None,
            error_message: None,
        }
    }
}

struct ModemConnection {
    port: Option<Box<dyn SerialPort>>,
    modem_type: ModemType,
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
        }));

        let rfd_connection = Arc::new(Mutex::new(ModemConnection {
            port: None,
            modem_type: ModemType::Rfd900x,
        }));

        let instance = Self {
            lora_connection,
            rfd_connection,
            lora_status: Arc::new(Mutex::new(ModemStatus::new())),
            rfd_status: Arc::new(Mutex::new(ModemStatus::new())),
            scan_thread: None,
            scan_running: Arc::new(Mutex::new(false)),
            lora_enabled: Arc::new(Mutex::new(true)),
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

                // Check if connections are still alive
                if now.duration_since(last_connection_check).as_millis()
                    >= CONNECTION_CHECK_INTERVAL_MS as u128
                {
                    // Check LoRa connection
                    if *lora_enabled.lock().unwrap() {
                        let mut lora_conn = lora_connection.lock().unwrap();
                        let mut status = lora_status.lock().unwrap();

                        if status.connected {
                            // Verify LoRa connection is still active
                            if let Some(port) = &mut lora_conn.port {
                                if let Err(e) = Self::verify_lora_connection(port) {
                                    // Connection failed
                                    status.connected = false;
                                    status.error_message = Some(format!("Connection lost: {}", e));
                                    lora_conn.port = None;
                                    println!("LoRa connection lost: {}", e);
                                } else {
                                    status.last_success = Some(Instant::now());
                                }
                            } else {
                                // Port is gone somehow
                                status.connected = false;
                                status.error_message = Some("Port handle lost".to_string());
                            }
                        }
                    }

                    // Check RFD connection
                    if *rfd_enabled.lock().unwrap() {
                        let mut rfd_conn = rfd_connection.lock().unwrap();
                        let mut status = rfd_status.lock().unwrap();

                        if status.connected {
                            // Verify RFD connection is still active
                            if let Some(port) = &mut rfd_conn.port {
                                if let Err(e) = Self::verify_rfd_connection(port) {
                                    // Connection failed
                                    status.connected = false;
                                    status.error_message = Some(format!("Connection lost: {}", e));
                                    rfd_conn.port = None;
                                    println!("RFD 900x2 connection lost: {}", e);
                                } else {
                                    status.last_success = Some(Instant::now());
                                }
                            } else {
                                // Port is gone somehow
                                status.connected = false;
                                status.error_message = Some("Port handle lost".to_string());
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

                            stat.connected = true;
                            stat.port_name = Some(port_name.clone());
                            stat.last_success = Some(Instant::now());
                            stat.error_message = None;

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
                state_change: 0, // Assuming 0 is a valid default
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

    // Verify an RFD connection using AT commands - IMPROVED VERSION
    fn verify_rfd_connection(port: &mut Box<dyn SerialPort>) -> Result<(), String> {
        // Set timeout for all operations
        port.set_timeout(Duration::from_millis(1000))
            .map_err(|e| format!("Failed to set timeout: {}", e))?;

        // Clear any pending data and flush
        let _ = port.flush();

        // Clear input buffer
        let mut clear_buf = [0u8; 256];
        while port.read(&mut clear_buf).is_ok() {
            // Keep reading until buffer is empty
        }

        // Wait a bit before sending +++
        thread::sleep(Duration::from_millis(500));

        // Send +++ to enter AT command mode (don't send \r\n after +++)
        port.write_all(b"+++")
            .map_err(|e| format!("Failed to send +++: {}", e))?;

        // Wait for AT mode (RFD docs suggest 1 second)
        thread::sleep(Duration::from_millis(1000));

        // Try to read any response to +++
        let mut response_buf = [0u8; 64];
        match port.read(&mut response_buf) {
            Ok(n) if n > 0 => {
                let response = String::from_utf8_lossy(&response_buf[..n]);
                println!("RFD response to +++: '{}'", response.trim());
            }
            _ => {
                println!("No immediate response to +++, continuing...");
            }
        }

        // Send AT command to check if we're in command mode
        port.write_all(b"ATI\r\n")
            .map_err(|e| format!("Failed to send ATI: {}", e))?;

        // Wait for response
        thread::sleep(Duration::from_millis(500));

        // Read response
        let mut response = vec![0u8; 256];
        match port.read(&mut response) {
            Ok(n) if n > 0 => {
                let response_str = String::from_utf8_lossy(&response[..n]);
                println!("RFD ATI response: '{}'", response_str.trim());

                // Check if response contains typical RFD firmware info
                if response_str.contains("RFD")
                    || response_str.contains("SiK")
                    || response_str.contains("OK")
                {
                    // Send ATO to exit AT mode and return to operational mode
                    port.write_all(b"ATO\r\n")
                        .map_err(|e| format!("Failed to send ATO: {}", e))?;
                    thread::sleep(Duration::from_millis(100));

                    println!("RFD 900x2 verification successful");
                    Ok(())
                } else {
                    Err(format!("Invalid RFD response: {}", response_str.trim()))
                }
            }
            Ok(_) => Err("No response to ATI command".to_string()),
            Err(e) => Err(format!("Failed to read ATI response: {}", e)),
        }
    }

    // Send CAN frame to all enabled and connected modems
    pub fn send_can_frame(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        let lora_connected = self.lora_status.lock().unwrap().connected;
        let rfd_connected = self.rfd_status.lock().unwrap().connected;
        let lora_enabled = self.is_lora_enabled();
        let rfd_enabled = self.is_rfd_enabled();

        // Collect errors to return a combined result
        let mut errors = Vec::new();
        let mut success_count = 0;

        if lora_enabled && lora_connected {
            match self.send_can_frame_lora(can_id, data) {
                Ok(_) => success_count += 1,
                Err(e) => errors.push(format!("LoRa error: {}", e)),
            }
        }

        if rfd_enabled && rfd_connected {
            match self.send_can_frame_rfd(can_id, data) {
                Ok(_) => success_count += 1,
                Err(e) => errors.push(format!("RFD error: {}", e)),
            }
        }

        if success_count > 0 && errors.is_empty() {
            Ok(())
        } else if success_count > 0 {
            // Some succeeded, some failed - log but don't fail completely
            println!("Partial modem transmission failure: {}", errors.join("; "));
            Ok(())
        } else if !errors.is_empty() {
            Err(errors.join("; "))
        } else {
            Err("No modems available for transmission".to_string())
        }
    }

    // Send CAN frame to LoRa modem
    fn send_can_frame_lora(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        // Quick check if port exists to fail fast
        let port_exists = {
            let guard = self
                .lora_connection
                .lock()
                .map_err(|_| "Failed to lock port mutex".to_string())?;
            guard.port.is_some()
        };

        if !port_exists {
            return Err("LoRa port not open".to_string());
        }

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

        // Properly frame the message with start and end delimiters
        let mut framed_data =
            Vec::with_capacity(START_DELIMITER.len() + encoded.len() + END_DELIMITER.len());
        framed_data.extend_from_slice(START_DELIMITER);
        framed_data.extend_from_slice(&encoded);
        framed_data.extend_from_slice(END_DELIMITER);

        // Get a locked reference to the port
        let mut lora_conn = match self.lora_connection.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Port is busy - log but don't block
                return Ok(()); // Return Ok to prevent error flooding
            }
        };

        // Write to serial port with timeout handling
        if let Some(port) = lora_conn.port.as_mut() {
            // Try to flush but don't fail if it doesn't work
            let _ = port.flush();

            // Write data with timeout protection
            match port.write_all(&framed_data) {
                Ok(_) => {
                    // Try to flush but don't fail if it doesn't work
                    let _ = port.flush();
                    Ok(())
                }
                Err(e) => Err(format!("Failed to write to LoRa port: {}", e)),
            }
        } else {
            Err("LoRa port not open".to_string())
        }
    }

    // Send CAN frame to RFD modem
    fn send_can_frame_rfd(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        // Quick check if port exists to fail fast
        let port_exists = {
            let guard = self
                .rfd_connection
                .lock()
                .map_err(|_| "Failed to lock port mutex".to_string())?;
            guard.port.is_some()
        };

        if !port_exists {
            return Err("RFD port not open".to_string());
        }

        // For RFD 900x2, we send the raw CAN ID followed by the data
        // This will be transmitted as-is to the remote end
        let mut payload = Vec::with_capacity(4 + data.len());
        payload.extend_from_slice(&can_id.to_be_bytes());
        payload.extend_from_slice(data);

        // Get a locked reference to the port
        let mut rfd_conn = match self.rfd_connection.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Port is busy - log but don't block
                return Ok(()); // Return Ok to prevent error flooding
            }
        };

        // Write to serial port with timeout handling
        if let Some(port) = rfd_conn.port.as_mut() {
            // Try to flush but don't fail if it doesn't work
            let _ = port.flush();

            // Write data with timeout protection
            match port.write_all(&payload) {
                Ok(_) => {
                    // Try to flush but don't fail if it doesn't work
                    let _ = port.flush();
                    Ok(())
                }
                Err(e) => Err(format!("Failed to write to RFD port: {}", e)),
            }
        } else {
            Err("RFD port not open".to_string())
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
