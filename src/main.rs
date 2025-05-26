mod can;
mod gui;
mod gui_modules;
mod logger;
mod proto;
mod serial;

use clap::{Arg, Command};
use gui::TelemetryGui;
use iced::{Application, Settings};

fn main() -> iced::Result {
    let matches = Command::new("telemetry-rs")
        .version("0.1.0")
        .author("Your Name")
        .about("Telemetry application with CAN bus and radio support")
        .arg(
            Arg::new("disable-lora")
                .long("disable-lora")
                .help("Disable LoRa modem")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("disable-rfd")
                .long("disable-rfd")
                .help("Disable RFD 900x2 modem")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("lora-only")
                .long("lora-only")
                .help("Enable only LoRa modem (disables RFD)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("rfd-only")
                .long("rfd-only")
                .help("Enable only RFD 900x2 modem (disables LoRa)")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let lora_enabled = !matches.get_flag("disable-lora") && !matches.get_flag("rfd-only");
    let rfd_enabled = !matches.get_flag("disable-rfd") && !matches.get_flag("lora-only");

    println!("Starting Telemetry Application");
    println!(
        "LoRa modem: {}",
        if lora_enabled { "ENABLED" } else { "DISABLED" }
    );
    println!(
        "RFD 900x2 modem: {}",
        if rfd_enabled { "ENABLED" } else { "DISABLED" }
    );

    let settings = Settings {
        flags: (lora_enabled, rfd_enabled),
        ..Settings::default()
    };

    TelemetryGui::run(settings)
}
