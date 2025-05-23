use chrono::{DateTime, Utc};
use iced::{widget::container::StyleSheet, Color, Theme};
use socketcan::CanFrame;
use std::collections::HashMap;

// Re-export common types and messages for all components

#[derive(Clone, Debug)]
pub struct Fault {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub is_active: bool,
    pub value: String,
    pub message_name: String, // Added to track which CAN message this came from
}

// Message enum shared between all components
#[derive(Debug, Clone)]
pub enum Message {
    CanFrameReceived(String, CanFrame),
    ToggleFullscreen,
    Tick, // For updating time display
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

// DTC fault definitions for BMS
pub const DTC_FLAGS_1_FAULTS: &[(u16, &str)] = &[
    (0x0001, "P0A07: Discharge Limit Enforcement"),
    (0x0002, "P0A08: Charger Safety Relay"),
    (0x0004, "P0A09: Internal Hardware"),
    (0x0008, "P0A0A: Internal Heatsink Thermistor"),
    (0x0010, "P0A0B: Internal Software"),
    (0x0020, "P0A0C: Highest Cell Voltage Too High"),
    (0x0040, "P0A0E: Lowest Cell Voltage Too Low"),
    (0x0080, "P0A10: Pack Too Hot"),
];

pub const DTC_FLAGS_2_FAULTS: &[(u16, &str)] = &[
    (0x0001, "P0A1F: Internal Communication"),
    (0x0002, "P0A12: Cell Balancing Stuck Off"),
    (0x0004, "P0A80: Weak Cell"),
    (0x0008, "P0AFA: Low Cell Voltage"),
    (0x0010, "P0A04: Open Wiring"),
    (0x0020, "P0AC0: Current Sensor"),
    (0x0040, "P0A0D: Highest Cell Voltage Over 5V"),
    (0x0080, "P0A0F: Cell ASIC"),
    (0x0100, "P0A02: Weak Pack"),
    (0x0200, "P0A81: Fan Monitor"),
    (0x0400, "P0A9C: Thermistor"),
    (0x0800, "U0100: External Communication"),
    (0x1000, "P0560: Redundant Power Supply"),
    (0x2000, "P0AA6: High Voltage Isolation"),
    (0x4000, "P0A05: Input Power Supply"),
    (0x8000, "P0A06: Charge Limit Enforcement"),
];
