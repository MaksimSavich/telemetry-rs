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

        Some(message.message_name().to_string())
    }
}
