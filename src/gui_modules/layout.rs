use crate::gui_modules::Message;
use iced::widget::{column, container, row, text};
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
pub fn main_layout(
    is_fullscreen: bool,
    modem_status_element: Element<'static, Message>,
    direction_element: Element<'static, Message>,
    speed_element: Element<'static, Message>,
    status_element: Element<'static, Message>,
    battery_element: Element<'static, Message>,
    bps_element: Element<'static, Message>,
    serial_element: Element<'static, Message>,
    fault_element: Element<'static, Message>,
) -> Element<'static, Message> {
    column![
        // Top row with fullscreen button and modem status
        row![fullscreen_button(is_fullscreen), modem_status_element],
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
