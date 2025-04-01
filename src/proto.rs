pub mod packet {
    include!(concat!(env!("OUT_DIR"), "/lora.packet.rs"));
}

pub use packet::*;
