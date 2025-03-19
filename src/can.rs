pub use socketcan::{CanSocket, EmbeddedFrame, Socket, StandardId};
use std::error::Error;
use std::io::ErrorKind;
use std::time::Duration;

pub fn can_test_reader(interface: &str) -> Result<(), Box<dyn Error>> {
    // Open the CAN socket
    let socket = CanSocket::open(interface)?;

    // Set to non-blocking mode
    socket.set_nonblocking(true)?;

    println!("Listening on CAN interface: {}", interface);

    loop {
        match socket.read_frame() {
            Ok(frame) => {
                let raw_id: u32 = match frame.id() {
                    socketcan::Id::Standard(std_id) => std_id.as_raw() as u32,
                    socketcan::Id::Extended(ext_id) => ext_id.as_raw(),
                };
                // Format data as hexadecimal
                let data_hex: Vec<String> = frame
                    .data()
                    .iter()
                    .map(|byte| format!("{:02X}", byte))
                    .collect();

                println!(
                    "Frame received: ID={:03X} Data=[{}]",
                    raw_id,
                    data_hex.join(" ")
                );
            }
            Err(e) => {
                // EAGAIN/EWOULDBLOCK handling - these are expected in non-blocking mode
                if e.kind() == ErrorKind::WouldBlock || e.raw_os_error() == Some(11) {
                    // This is normal for non-blocking sockets - no data available yet
                    // Add a small sleep to avoid busy-waiting and high CPU usage
                    std::thread::sleep(Duration::from_millis(10));
                    continue;
                } else {
                    // Other errors might be actual problems
                    eprintln!("Error receiving CAN frame: {}", e);
                    return Err(e.into());
                }
            }
        }
    }
}
