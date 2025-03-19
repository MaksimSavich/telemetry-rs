mod can;
use std::env;

fn main() {
    // Default to vcan0 if no argument is provided
    let interface = env::args().nth(1).unwrap_or_else(|| "vcan0".to_string());

    println!("Starting CAN reader on interface: {}", interface);

    if let Err(e) = can::can_test_reader(&interface) {
        eprintln!("Error: {}", e);
    }
}
