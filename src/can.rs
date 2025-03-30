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
        let message = self
            .dbc
            .messages()
            .iter()
            .find(|m| m.message_id().raw() == raw_id)?;

        Some(
            message
                .signals()
                .iter()
                .fold(String::new(), |mut acc, signal| {
                    let raw_value = {
                        let data = {
                            let mut data_array = frame.data().to_vec();
                            data_array.reverse(); // Reverse if needed based on your CAN implementation
                            data_array
                        };
                        let start_bit = *signal.start_bit() as usize;
                        let size = *signal.signal_size() as usize;

                        // Determine endianness from the DBC signal
                        // In DBC format, Intel (little endian) is usually marked as "1"
                        // Motorola (big endian) is usually marked as "0"
                        let is_intel = match signal.byte_order() {
                            can_dbc::ByteOrder::LittleEndian => true,
                            can_dbc::ByteOrder::BigEndian => false,
                        };

                        self.extract_signal_value(&data, start_bit, size, is_intel)
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

    // Helper method to extract signal value based on endianness
    fn extract_signal_value(
        &self,
        data: &[u8],
        start_bit: usize,
        size: usize,
        is_intel: bool,
    ) -> u64 {
        let mut value = 0u64;

        if is_intel {
            // Intel format (little-endian)
            for i in 0..size {
                let byte_index = (start_bit + i) / 8;
                let bit_index = (start_bit + i) % 8;

                // Make sure we don't go out of bounds
                if byte_index < data.len() {
                    if (data[byte_index] & (1 << bit_index)) != 0 {
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
                    if (data[current_byte] & (1 << current_bit_in_byte)) != 0 {
                        value |= 1 << (size - 1 - i);
                    }
                }
            }
        }

        value
    }
}
