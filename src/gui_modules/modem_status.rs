// New file: src/gui_modules/modem_status.rs
use crate::gui_modules::Message;
use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length};

#[derive(Clone)]
pub struct ModemStatusData {
    pub rfd_connected: bool,
    pub lora_connected: bool,
}

pub fn modem_status(data: &ModemStatusData) -> Element<'static, Message> {
    let rfd_text = if data.rfd_connected {
        "RFD: Connected"
    } else {
        "RFD: Disconnected"
    };

    let lora_text = if data.lora_connected {
        "LoRa: Connected"
    } else {
        "LoRa: Disconnected"
    };

    container(
        row![
            text(rfd_text)
                .width(Length::Fill)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
            text(lora_text)
                .width(Length::Fill)
                .horizontal_alignment(iced::alignment::Horizontal::Center)
        ]
        .spacing(10)
        .align_items(Alignment::Center),
    )
    .padding(5)
    .width(Length::Fill)
    .style(iced::theme::Container::Box)
    .into()
}
