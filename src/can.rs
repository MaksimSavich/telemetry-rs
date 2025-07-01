// Fixed src/can.rs - Updated CAN signal extraction with proper signed/unsigned handling

use crate::gui_modules::{DTC_FLAGS_1_FAULTS, DTC_FLAGS_2_FAULTS};
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

        // Special handling for DTC flags message (ID 0x300)
        if raw_id == 0x300 && frame.data().len() >= 4 {
            return Some(self.decode_dtc_flags(frame.data()));
        }

        // First, check for exact match with raw_id
        let exact_match = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            msg_id == raw_id
        });

        if let Some(message) = exact_match {
            return self.decode_message(message, frame);
        }

        // Try with 11-bit ID (masked with 0x7FF for standard IDs)
        let standard_id = raw_id & 0x7FF;
        let standard_match = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            msg_id == standard_id
        });

        if let Some(message) = standard_match {
            return self.decode_message(message, frame);
        }

        // Try with 29-bit ID (masked with 0x1FFFFFFF for extended IDs)
        let extended_id = raw_id & 0x1FFFFFFF;
        let extended_match = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            msg_id == extended_id
        });

        if let Some(message) = extended_match {
            return self.decode_message(message, frame);
        }

        // Check if the DBC might have extended flag set (0x80000000) that we need to add
        let flag_extended_id = raw_id | 0x80000000;
        let flag_match = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            msg_id == flag_extended_id
        });

        if let Some(message) = flag_match {
            return self.decode_message(message, frame);
        }

        // One more try for MotorController messages specifically - look for the right message pattern
        if (raw_id & 0xFFFFFF00) == 0x8CF11E00 || (raw_id & 0x1FFFFFFF) == 0x0CF11E05 {
            let mc_match = self.dbc.messages().iter().find(|m| {
                let msg_id = m.message_id().raw();
                msg_id == 0x8CF11E05 || msg_id == 217128453
            });

            if let Some(message) = mc_match {
                return self.decode_message(message, frame);
            }
        }

        if (raw_id & 0xFFFFFF00) == 0x8CF11F00 || (raw_id & 0x1FFFFFFF) == 0x0CF11F05 {
            let mc_match = self.dbc.messages().iter().find(|m| {
                let msg_id = m.message_id().raw();
                msg_id == 0x8CF11F05 || msg_id == 217128709
            });

            if let Some(message) = mc_match {
                return self.decode_message(message, frame);
            }
        }

        None
    }

    fn decode_dtc_flags(&self, data: &[u8]) -> String {
        let mut result = String::new();

        // Extract DTC_Flags_1 (first 2 bytes, little endian)
        if data.len() >= 2 {
            let flags1 = u16::from_le_bytes([data[0], data[1]]);

            for (mask, fault_name) in DTC_FLAGS_1_FAULTS {
                if flags1 & mask != 0 {
                    result.push_str(&format!(
                        "Fault_DTC1_{}: {}\n",
                        fault_name.split(':').next().unwrap(),
                        fault_name
                    ));
                }
            }
        }

        // Extract DTC_Flags_2 (next 2 bytes, little endian)
        if data.len() >= 4 {
            let flags2 = u16::from_le_bytes([data[2], data[3]]);

            for (mask, fault_name) in DTC_FLAGS_2_FAULTS {
                if flags2 & mask != 0 {
                    result.push_str(&format!(
                        "Fault_DTC2_{}: {}\n",
                        fault_name.split(':').next().unwrap(),
                        fault_name
                    ));
                }
            }
        }

        if result.is_empty() {
            result.push_str("DTC_Flags_1: 0\nDTC_Flags_2: 0\n");
        }

        result
    }

    fn decode_message(&self, message: &can_dbc::Message, frame: CanFrame) -> Option<String> {
        Some(
            message
                .signals()
                .iter()
                .fold(String::new(), |mut acc, signal| {
                    let raw_value = {
                        let data_array = frame.data().to_vec();

                        let start_bit = *signal.start_bit() as usize;
                        let size = *signal.signal_size() as usize;

                        // Determine endianness from the DBC signal
                        let is_intel = match signal.byte_order() {
                            can_dbc::ByteOrder::LittleEndian => true,
                            can_dbc::ByteOrder::BigEndian => false,
                        };

                        // Determine if signal is signed by checking the minimum value
                        // This is the most reliable way since DBC min values indicate signedness
                        let is_signed = *signal.min() < 0.0;

                        self.extract_signal_value(&data_array, start_bit, size, is_intel, is_signed)
                    };

                    // Scale raw value to engineering value
                    let signal_value = (*signal.factor() * raw_value as f64) + *signal.offset();

                    // Lookup value descriptions via DBC
                    let value_desc = self
                        .dbc
                        .value_descriptions_for_signal(*message.message_id(), signal.name())
                        .and_then(|descs| {
                            descs
                                .iter()
                                .find(|desc| (*desc.a()) as i64 == raw_value)
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
        is_signed: bool,
    ) -> i64 {
        // Validate input parameters
        if size == 0 || size > 64 {
            return 0;
        }

        let mut raw_value = 0u64;

        if is_intel {
            // Intel format (little-endian)
            for i in 0..size {
                let byte_index = (start_bit + i) / 8;
                let bit_index = (start_bit + i) % 8;

                if byte_index < data.len() {
                    let bit_value = (data[byte_index] & (1 << bit_index)) != 0;
                    if bit_value {
                        raw_value |= 1 << i;
                    }
                }
            }
        } else {
            // Motorola format (big-endian)
            for i in 0..size {
                let bit_pos = start_bit - i;
                let byte_index = bit_pos / 8;
                let bit_index = 7 - (bit_pos % 8);

                if byte_index < data.len() {
                    let bit_value = (data[byte_index] & (1 << bit_index)) != 0;
                    if bit_value {
                        raw_value |= 1 << (size - 1 - i);
                    }
                }
            }
        }

        // Handle sign extension for signed values using two's complement
        if is_signed && size > 0 && size <= 64 {
            // Check if the sign bit (MSB) is set
            let sign_bit = 1u64 << (size - 1);
            if raw_value & sign_bit != 0 {
                // Negative number: perform sign extension
                // Create a mask with all upper bits set to 1
                let mask = (!0u64) << size;
                // Apply the mask to extend the sign bit
                (raw_value | mask) as i64
            } else {
                // Positive number: just cast to signed
                raw_value as i64
            }
        } else {
            // Unsigned value: cast to signed (will be positive)
            raw_value as i64
        }
    }
}
