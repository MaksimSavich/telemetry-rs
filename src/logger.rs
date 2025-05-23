use chrono::{DateTime, Local};
use socketcan::{CanFrame, EmbeddedFrame};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct CanLogger {
    log_file: File,
    log_path: PathBuf,
}

impl CanLogger {
    pub fn new() -> Result<Self, std::io::Error> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("log_{}.txt", timestamp);
        let log_path = PathBuf::from(&filename);

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_path)?;

        // Write header
        writeln!(file, "# CAN Log Started: {}", Local::now())?;
        writeln!(file, "# Format: TIMESTAMP ARBITRATION_ID MESSAGE_DATA_HEX")?;
        writeln!(file, "#")?;

        Ok(Self {
            log_file: file,
            log_path,
        })
    }

    pub fn log_frame(&mut self, frame: &CanFrame) -> Result<(), std::io::Error> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S.%3f");

        let id = match frame.id() {
            socketcan::Id::Standard(std_id) => format!("0x{:03X}", std_id.as_raw()),
            socketcan::Id::Extended(ext_id) => format!("0x{:08X}", ext_id.as_raw()),
        };

        let data_hex = frame
            .data()
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        writeln!(self.log_file, "{} {} {}", timestamp, id, data_hex)?;
        self.log_file.flush()?;

        Ok(())
    }

    pub fn get_log_path(&self) -> &PathBuf {
        &self.log_path
    }
}
