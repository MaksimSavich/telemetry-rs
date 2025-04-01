use crate::gui_modules::Message;
use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length};

#[derive(Clone)]
pub struct BpsData {
    pub ontime: u64,
    pub state: String,
}

pub fn bps_box(data: &BpsData) -> Element<'static, Message> {
    container(
        column![
            text("BPS Info").size(20),
            text(format!("Time: {:.1} Seconds", data.ontime)),
            text(format!("State: {}", data.state)),
        ]
        .spacing(5)
        .align_items(Alignment::Start),
    )
    .padding(10)
    .width(Length::FillPortion(1))
    .style(iced::theme::Container::Box)
    .into()
}
