use can_dbc::DBC;
use socketcan::{CanFrame, EmbeddedFrame};
use std::fs;

#[derive(Clone)]
pub struct CanDecoder {
    dbc: DBC,
}

impl CanDecoder {
    pub fn new(dbc_path: &str) -> Self {
        let dbc_content = fs::read_to_string(dbc_path).expect("Failed to read DBC file");
        let dbc = DBC::from_slice(dbc_content.as_bytes()).expect("Failed to parse DBC");
        Self { dbc }
    }

    pub fn decode(&self, frame: CanFrame) -> Option<String> {
        // Get the raw ID without any modification first
        let raw_id = match frame.id() {
            socketcan::Id::Standard(std_id) => std_id.as_raw() as u32,
            socketcan::Id::Extended(ext_id) => ext_id.as_raw(),
        };

        // Add debug logging
        println!(
            "Decoding frame with ID: 0x{:X}, data: {:?}",
            raw_id,
            frame.data()
        );

        // Print all messages in the DBC for debugging
        println!("Looking for ID 0x{:X} in DBC:", raw_id);
        let mut closest_id = None;
        let mut min_diff = u32::MAX;

        // First, check for exact match with raw_id
        let exact_match = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            println!("  DBC message ID: 0x{:X}", msg_id);

            // Find the closest ID for fallback
            let diff = if msg_id > raw_id {
                msg_id - raw_id
            } else {
                raw_id - msg_id
            };
            if diff < min_diff {
                min_diff = diff;
                closest_id = Some(msg_id);
            }

            msg_id == raw_id
        });

        if let Some(message) = exact_match {
            println!("  Found exact match: 0x{:X}", message.message_id().raw());
            return self.decode_message(message, frame);
        }

        // Try with 11-bit ID (masked with 0x7FF for standard IDs)
        let standard_id = raw_id & 0x7FF;
        let standard_match = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            msg_id == standard_id
        });

        if let Some(message) = standard_match {
            println!(
                "  Found standard ID match: 0x{:X}",
                message.message_id().raw()
            );
            return self.decode_message(message, frame);
        }

        // Try with 29-bit ID (masked with 0x1FFFFFFF for extended IDs)
        let extended_id = raw_id & 0x1FFFFFFF;
        let extended_match = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            msg_id == extended_id
        });

        if let Some(message) = extended_match {
            println!(
                "  Found extended ID match: 0x{:X}",
                message.message_id().raw()
            );
            return self.decode_message(message, frame);
        }

        // Check if the DBC might have extended flag set (0x80000000) that we need to add
        let flag_extended_id = raw_id | 0x80000000;
        let flag_match = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            msg_id == flag_extended_id
        });

        if let Some(message) = flag_match {
            println!(
                "  Found extended flag match: 0x{:X}",
                message.message_id().raw()
            );
            return self.decode_message(message, frame);
        }

        // One more try for MotorController messages specifically - look for the right message pattern
        if (raw_id & 0xFFFFFF00) == 0x8CF11E00 || (raw_id & 0x1FFFFFFF) == 0x0CF11E05 {
            let mc_match = self.dbc.messages().iter().find(|m| {
                let msg_id = m.message_id().raw();
                msg_id == 0x8CF11E05 || msg_id == 217128453
            });

            if let Some(message) = mc_match {
                println!(
                    "  Found MotorController match: 0x{:X}",
                    message.message_id().raw()
                );
                return self.decode_message(message, frame);
            }
        }

        if (raw_id & 0xFFFFFF00) == 0x8CF11F00 || (raw_id & 0x1FFFFFFF) == 0x0CF11F05 {
            let mc_match = self.dbc.messages().iter().find(|m| {
                let msg_id = m.message_id().raw();
                msg_id == 0x8CF11F05 || msg_id == 217128709
            });

            if let Some(message) = mc_match {
                println!(
                    "  Found MotorController match: 0x{:X}",
                    message.message_id().raw()
                );
                return self.decode_message(message, frame);
            }
        }

        println!("  No matching message found in DBC for ID: 0x{:X}", raw_id);
        println!(
            "  Closest ID was: 0x{:X} (diff: {})",
            closest_id.unwrap_or(0),
            min_diff
        );

        None
    }

    fn decode_message(&self, message: &can_dbc::Message, frame: CanFrame) -> Option<String> {
        Some(
            message
                .signals()
                .iter()
                .fold(String::new(), |mut acc, signal| {
                    let raw_value = {
                        // Re-add the data array reversal that was necessary for correct decoding
                        let data_array = frame.data().to_vec();
                        // data_array.reverse(); // Re-add the reversal for your specific CAN implementation

                        let start_bit = *signal.start_bit() as usize;
                        let size = *signal.signal_size() as usize;

                        // Determine endianness from the DBC signal
                        let is_intel = match signal.byte_order() {
                            can_dbc::ByteOrder::LittleEndian => true,
                            can_dbc::ByteOrder::BigEndian => false,
                        };

                        self.extract_signal_value(&data_array, start_bit, size, is_intel)
                    };

                    // Scale raw value to engineering value
                    let signal_value = (*signal.factor() * raw_value as f64) + *signal.offset();

                    // Debug the extraction
                    println!(
                        "  Extracted signal: {}, raw: {}, scaled: {}",
                        signal.name(),
                        raw_value,
                        signal_value
                    );

                    // Lookup value descriptions via DBC
                    let value_desc = self
                        .dbc
                        .value_descriptions_for_signal(*message.message_id(), signal.name())
                        .and_then(|descs| {
                            descs
                                .iter()
                                .find(|desc| (*desc.a()) as u64 == raw_value)
                                .map(|d| d.b())
                        });

                    if let Some(desc) = value_desc {
                        acc.push_str(&format!("{}: {}\n", signal.name(), desc));
                    } else {
                        acc.push_str(&format!("{}: {}\n", signal.name(), signal_value));
                    }
                    acc
                }),
        )
    }

    fn extract_signal_value(
        &self,
        data: &[u8],
        start_bit: usize,
        size: usize,
        is_intel: bool,
    ) -> u64 {
        let mut value = 0u64;

        println!(
            "Extracting signal: start_bit={}, size={}, is_intel={}",
            start_bit, size, is_intel
        );
        println!("Data bytes: {:02X?}", data); // Print all data bytes in hex

        if is_intel {
            // Intel format (little-endian)
            for i in 0..size {
                let byte_index = (start_bit + i) / 8;
                let bit_index = (start_bit + i) % 8;

                // Make sure we don't go out of bounds
                if byte_index < data.len() {
                    let bit_value = (data[byte_index] & (1 << bit_index)) != 0;
                    println!(
                        "  Little-endian bit {}: byte_index={}, bit_index={}, bit value={}",
                        i, byte_index, bit_index, bit_value
                    );

                    if bit_value {
                        value |= 1 << i;
                    }
                }
            }
        } else {
            // Motorola format (big-endian)
            // In Motorola format, bits are numbered from MSB to LSB
            for i in 0..size {
                // Calculate the actual bit position in the CAN frame
                let byte_index = start_bit / 8;
                let bit_index = 7 - (start_bit % 8);
                let current_bit = start_bit + i;
                let current_byte = byte_index - (current_bit / 8 - byte_index);
                let current_bit_in_byte = 7 - ((bit_index - i) % 8);

                // Make sure we don't go out of bounds
                if current_byte < data.len() {
                    let bit_value = (data[current_byte] & (1 << current_bit_in_byte)) != 0;
                    println!(
                        "  Big-endian bit {}: byte={}, bit_in_byte={}, bit value={}",
                        i, current_byte, current_bit_in_byte, bit_value
                    );

                    if bit_value {
                        value |= 1 << (size - 1 - i);
                    }
                }
            }
        }

        println!("Extracted value: {}", value);
        value
    }
}
