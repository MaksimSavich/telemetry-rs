mod can;
mod gui;

use gui::TelemetryGui;
use iced::{Application, Settings}; // <-- change here (use Application)

fn main() -> iced::Result {
    TelemetryGui::run(Settings::default())
}
