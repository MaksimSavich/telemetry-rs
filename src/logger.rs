use chrono::Local;
use socketcan::{CanFrame, EmbeddedFrame};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct CanLogger {
    log_file: File,
    log_path: PathBuf,
}

impl CanLogger {
    pub fn new() -> Result<Self, std::io::Error> {
        // Clean up old logs if total size exceeds 10GB
        Self::cleanup_logs_if_needed()?;

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

    fn cleanup_logs_if_needed() -> Result<(), std::io::Error> {
        const MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024 * 1024; // 10GB

        let current_dir = std::env::current_dir()?;
        let mut log_files = Vec::new();
        let mut total_size = 0u64;

        // Find all log files and calculate total size
        for entry in fs::read_dir(&current_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("log_") && filename.ends_with(".txt") {
                    let metadata = entry.metadata()?;
                    let size = metadata.len();
                    total_size += size;

                    log_files.push((path, metadata.modified()?));
                }
            }
        }

        // If total size exceeds limit, delete oldest files
        if total_size > MAX_SIZE_BYTES {
            // Sort by modification time (oldest first)
            log_files.sort_by_key(|(_, modified)| *modified);

            let mut removed_size = 0u64;
            for (path, _) in log_files {
                if total_size - removed_size <= MAX_SIZE_BYTES {
                    break;
                }

                if let Ok(metadata) = fs::metadata(&path) {
                    removed_size += metadata.len();
                    let _ = fs::remove_file(&path);
                }
            }
        }

        Ok(())
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
