use prost::Message;
use serialport::{SerialPort, SerialPortType};
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::proto::{Packet, PacketType, Transmission};

// Define the start and end delimiters to match the LoRa module
const START_DELIMITER: &[u8] = b"<START>"; // Start delimiter
const END_DELIMITER: &[u8] = b"<END>"; // End delimiter

pub struct SerialManager {
    port: Arc<Mutex<Option<Box<dyn SerialPort>>>>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            port: Arc::new(Mutex::new(None)),
        }
    }

    pub fn connect(&self, port_name: &str, baud_rate: u32) -> Result<(), String> {
        let mut port_handle = self.port.lock().unwrap();

        // Close existing port if open
        *port_handle = None;

        // Try to open the new port
        match serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(100))
            .open()
        {
            Ok(port) => {
                *port_handle = Some(port);
                Ok(())
            }
            Err(e) => Err(format!("Failed to open serial port: {}", e)),
        }
    }

    pub fn send_can_frame(&self, can_id: u32, data: &[u8]) -> Result<(), String> {
        let mut port_guard = self.port.lock().unwrap();

        // Check if port is open
        if port_guard.is_none() {
            return Err("Serial port not open".to_string());
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

        // Add start delimiter
        framed_data.extend_from_slice(START_DELIMITER);

        // // Add length prefix for framing (2 bytes, big endian)
        // let len = encoded.len() as u16;
        // framed_data.extend_from_slice(&len.to_be_bytes());
        framed_data.extend_from_slice(&encoded);

        // Add end delimiter
        framed_data.extend_from_slice(END_DELIMITER);

        // Write to serial port
        if let Some(port) = port_guard.as_mut() {
            // Flush before writing to ensure clean state
            port.flush()
                .map_err(|e| format!("Failed to flush serial port: {}", e))?;

            port.write_all(&framed_data)
                .map_err(|e| format!("Failed to write to serial port: {}", e))?;

            // Ensure data is sent by flushing again
            port.flush()
                .map_err(|e| format!("Failed to flush after write: {}", e))?;

            // More detailed debugging
            println!(
                "Sent frame: ID=0x{:X}, data={:?}, framed_size={}, frame_bytes={:?}",
                can_id,
                data,
                framed_data.len(),
                framed_data
                    .iter()
                    .map(|b| format!("{:02X}", b))
                    .collect::<Vec<_>>()
                    .join(" ")
            );

            Ok(())
        } else {
            Err("Serial port not open".to_string())
        }
    }

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
            port: Arc::clone(&self.port),
        }
    }
}
