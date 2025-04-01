use crate::gui::types::Message;
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

// Add this to mod.rs
pub fn fullscreen_button(is_fullscreen: bool) -> Element<'static, Message> {
    container(
        iced::widget::button(text(if is_fullscreen {
            "Exit Fullscreen"
        } else {
            "Fullscreen"
        }))
        .on_press(Message::ToggleFullscreen),
    )
    .width(Length::Fill)
    .align_x(iced::alignment::Horizontal::Right)
    .into()
}

// Combine all parts into the main layout
pub fn main_layout<'a>(
    is_fullscreen: bool,
    direction_element: Element<'a, Message>,
    speed_element: Element<'a, Message>,
    status_element: Element<'a, Message>,
    battery_element: Element<'a, Message>,
    bps_element: Element<'a, Message>,
    serial_element: Element<'a, Message>,
    fault_element: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        // Top row for fullscreen button
        fullscreen_button(is_fullscreen),
        // Direction text
        direction_element,
        // Status, speed and battery info
        row![
            status_element,  // Left side
            speed_element,   // Center
            battery_element  // Right side
        ],
        bps_element,
        // Add Serial and LoRa section
        serial_element,
        // Add fault section
        fault_element
    ]
    .padding(20)
    .spacing(10)
    .width(Length::Fill)
    .height(Length::Fill)
    .align_items(Alignment::Center)
    .into()
}
