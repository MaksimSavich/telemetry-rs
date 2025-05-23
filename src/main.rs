mod can;
mod gui;
mod gui_modules;
mod logger;
mod proto;
mod serial;

use gui::TelemetryGui;
use iced::{Application, Settings};

fn main() -> iced::Result {
    TelemetryGui::run(Settings::default())
}
