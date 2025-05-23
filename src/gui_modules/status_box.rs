use crate::gui_modules::Message;
use iced::widget::container::StyleSheet;
use iced::widget::{column, container, row, text};
use iced::{Alignment, Color, Element, Length};

pub fn can_status_indicator(can_connected: bool) -> Element<'static, Message> {
    let status_text = if can_connected {
        "CAN: ONLINE"
    } else {
        "CAN: OFFLINE"
    };

    container(
        text(status_text)
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding(4)
    .width(Length::Fixed(100.0))
    .style(if can_connected {
        iced::theme::Container::Custom(Box::new(|theme: &iced::Theme| {
            let mut appearance = theme.appearance(&iced::theme::Container::Box);
            appearance.background = Some(Color::from_rgb(0.0, 0.8, 0.0).into());
            appearance.text_color = Some(Color::WHITE);
            appearance
        }))
    } else {
        iced::theme::Container::Custom(Box::new(|theme: &iced::Theme| {
            let mut appearance = theme.appearance(&iced::theme::Container::Box);
            appearance.background = Some(Color::from_rgb(0.8, 0.0, 0.0).into());
            appearance.text_color = Some(Color::WHITE);
            appearance
        }))
    })
    .into()
}

pub fn direction_speed_display(direction: &str, speed: f64) -> Element<'static, Message> {
    container(
        column![
            text(format!("{:.1}", speed))
                .size(72)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
            text("MPH")
                .size(24)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
            text(direction)
                .size(20)
                .horizontal_alignment(iced::alignment::Horizontal::Center)
        ]
        .spacing(0)
        .align_items(Alignment::Center)
        .width(Length::Fill),
    )
    .width(Length::FillPortion(1))
    .center_x()
    .center_y()
    .into()
}

pub fn time_display(current_time: &str) -> Element<'static, Message> {
    container(
        text(current_time)
            .size(16)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .center_x()
    .padding(5)
    .into()
}
