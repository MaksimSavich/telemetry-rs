use crate::gui_modules::Message;
use iced::widget::container::StyleSheet;
use iced::widget::{container, row, text};
use iced::{Alignment, Color, Element, Length};

pub fn radio_status_indicators(
    lora_connected: bool,
    rfd_connected: bool,
) -> Element<'static, Message> {
    let lora_box = container(
        text("LoRa")
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding(4)
    .width(Length::Fixed(50.0))
    .style(if lora_connected {
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
    });

    let rfd_box = container(
        text("RFD")
            .size(14)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .padding(4)
    .width(Length::Fixed(50.0))
    .style(if rfd_connected {
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
    });

    row![lora_box, rfd_box]
        .spacing(5)
        .align_items(Alignment::Center)
        .into()
}
