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
                        let data = frame.data();
                        let start_bit = *signal.start_bit() as usize;
                        let size = *signal.signal_size() as usize;
                        let mut value = 0u64;

                        for i in 0..size {
                            let byte_index = (start_bit + i) / 8;
                            let bit_index = (start_bit + i) % 8;
                            if (data[byte_index] & (1 << bit_index)) != 0 {
                                value |= 1 << i;
                            }
                        }
                        value
                    };
                    let signal_value = (*signal.factor() * raw_value as f64) + *signal.offset();
                    acc.push_str(&format!("{}: {}\n", signal.name(), signal_value));
                    acc
                }),
        )
    }
}
