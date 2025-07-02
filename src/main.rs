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
            Arg::new("disable-rfd")
                .long("disable-rfd")
                .help("Disable RFD 900x2 modem")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let rfd_enabled = !matches.get_flag("disable-rfd");

    println!("Starting Telemetry Application");
    println!(
        "RFD 900x2 modem: {}",
        if rfd_enabled { "ENABLED" } else { "DISABLED" }
    );

    let settings = Settings {
        flags: rfd_enabled,
        ..Settings::default()
    };

    TelemetryGui::run(settings)
}
