use chrono::{DateTime, Utc};
use iced::{widget::container::StyleSheet, Color, Theme};
use socketcan::CanFrame;

// Re-export common types and messages for all components

#[derive(Clone, Debug)]
pub struct Fault {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub is_active: bool,
    pub value: String,
}

// Message enum shared between all components
#[derive(Debug, Clone)]
pub enum Message {
    CanFrameReceived(String, CanFrame),
    ToggleFullscreen,
    ClearFaults,

    // Serial port messages
    PortSelected(String),
    ConnectSerialPort,
    ToggleLoRa,

    // Fault panel messages
    ToggleFaultPanelExpanded,
    CycleFault,
}

// Container styling helper
pub fn create_error_container_style() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(move |theme: &Theme| {
        let mut appearance = theme.appearance(&iced::theme::Container::Box);
        appearance.background = Some(Color::from_rgb(1.0, 0.0, 0.0).into());
        appearance.text_color = Some(Color::WHITE);
        appearance
    }))
}
