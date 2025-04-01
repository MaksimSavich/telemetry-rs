use crate::gui::types::Message;
use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length, Theme};

pub struct StatusData {
    pub direction: String,
    pub latest_fault: Option<String>,
}

pub fn status_box(data: &StatusData) -> Element<'static, Message> {
    container(
        column![
            text("CAN Status").size(20),
            text(format!("Direction: {}", data.direction)),
            text(format!(
                "Fault: {}",
                data.latest_fault.clone().unwrap_or("No Faults".into())
            ))
        ]
        .spacing(5)
        .align_items(Alignment::Start),
    )
    .padding(10)
    .width(Length::FillPortion(1))
    .style(iced::theme::Container::Box)
    .into()
}

pub fn direction_text(direction: &str) -> Element<'static, Message> {
    container(
        text(direction)
            .size(28)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .into()
}

pub fn speed_text(speed: f64) -> Element<'static, Message> {
    container(
        text(format!("{:.1} MPH", speed))
            .size(60)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .into()
}
