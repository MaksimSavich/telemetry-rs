mod can;
mod gui;

use gui::TelemetryGui;
use iced::{Application, Settings};

fn main() -> iced::Result {
    TelemetryGui::run(Settings::default())
}
