mod can_interface;
mod dbc;
mod gui;
mod telemetry; // Placeholder for future DBC handling

use can_interface::{simulated_stream, CanInterface};
use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Set to `true` to simulate input when hardware unavailable
    const SIMULATE_CAN: bool = true;

    if SIMULATE_CAN {
        println!("Starting simulated CAN stream...");
        let mut stream = simulated_stream();
        while let Some(frame) = stream.next().await {
            match frame {
                Ok(frame) => println!(
                    "Simulated CAN Frame: ID {:X}, Data: {:X?}",
                    frame.id(),
                    frame.data()
                ),
                Err(e) => eprintln!("Error: {:?}", e),
            }
        }
    } else {
        println!("Starting real CAN stream on interface can0...");
        let interface = CanInterface::new("can0")?;
        let mut stream = interface.stream();
        while let Some(frame) = stream.next().await {
            match frame {
                Ok(frame) => println!(
                    "Received CAN Frame: ID {:X}, Data: {:X?}",
                    frame.id(),
                    frame.data()
                ),
                Err(e) => eprintln!("CAN Read Error: {:?}", e),
            }
        }
    }

    Ok(())
}
