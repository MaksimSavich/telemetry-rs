use anyhow::Result;
use async_stream::stream;
use futures::Stream;
use socketcan::{CANFrame, CANSocket};
use std::time::Duration;
use tokio::time;

pub struct CanInterface {
    socket: CANSocket,
}

impl CanInterface {
    pub fn new(interface: &str) -> Result<Self> {
        let socket = CANSocket::open(interface)?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket })
    }

    pub fn stream(&self) -> impl Stream<Item = Result<CANFrame>> + '_ {
        stream! {
            loop {
                match self.socket.read_frame() {
                    Ok(frame) => yield Ok(frame),
                    Err(err) => {
                        if err.kind() != std::io::ErrorKind::WouldBlock {
                            yield Err(err.into());
                        }
                        // Wait briefly to avoid busy-looping
                        time::sleep(Duration::from_millis(1)).await;
                    }
                }
            }
        }
    }
}

// Simulation mode for testing without actual hardware
pub fn simulated_stream() -> impl Stream<Item = Result<CANFrame>> {
    stream! {
        loop {
            // Simulate a frame every 100ms
            let frame = CANFrame::new(0x123, &[0x11, 0x22, 0x33, 0x44], false, false)?;
            yield Ok(frame);
            time::sleep(Duration::from_millis(100)).await;
        }
    }
}
