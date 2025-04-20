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

        // Iterate through all messages to find match - be more lenient with matching
        let message = self.dbc.messages().iter().find(|m| {
            let msg_id = m.message_id().raw();
            // Print the ID being compared to help debug
            println!("  Checking against DBC message ID: 0x{:X}", msg_id);
            msg_id == raw_id
        });

        if message.is_none() {
            println!("  No matching message found in DBC for ID: 0x{:X}", raw_id);
            return None;
        }

        let message = message.unwrap();
        println!("  Found matching message: {}", message.name());

        Some(
            message
                .signals()
                .iter()
                .fold(String::new(), |mut acc, signal| {
                    let raw_value = {
                        let data = frame.data().to_vec();
                        let start_bit = *signal.start_bit() as usize;
                        let size = *signal.signal_size() as usize;

                        // Determine endianness from the DBC signal
                        let is_intel = match signal.byte_order() {
                            can_dbc::ByteOrder::LittleEndian => true,
                            can_dbc::ByteOrder::BigEndian => false,
                        };

                        self.extract_signal_value(&data, start_bit, size, is_intel)
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

        // Make a copy of data to ensure we don't modify the original
        let data_copy = data.to_vec();

        // DO NOT reverse the entire array - this was causing issues
        // Instead, handle endianness correctly in the bit extraction

        if is_intel {
            // Intel format (little-endian)
            for i in 0..size {
                let byte_index = (start_bit + i) / 8;
                let bit_index = (start_bit + i) % 8;

                // Make sure we don't go out of bounds
                if byte_index < data_copy.len() {
                    if (data_copy[byte_index] & (1 << bit_index)) != 0 {
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
                if current_byte < data_copy.len() {
                    if (data_copy[current_byte] & (1 << current_bit_in_byte)) != 0 {
                        value |= 1 << (size - 1 - i);
                    }
                }
            }
        }

        value
    }
}
