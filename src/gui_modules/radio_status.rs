use crate::gui_modules::Message;
use iced::widget::container::StyleSheet;
use iced::widget::{container, text};
use iced::{Color, Element, Length};

pub fn radio_status_indicators(
    rfd_connected: bool,
) -> Element<'static, Message> {

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

    rfd_box.into()
}
