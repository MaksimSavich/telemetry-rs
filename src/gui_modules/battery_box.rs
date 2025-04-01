use crate::gui::types::Message;
use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length, Theme};

pub struct BatteryData {
    pub voltage: f64,
    pub current: f64,
    pub charge: f64,
    pub temp: f64,
}

pub fn battery_box(data: &BatteryData) -> Element<'static, Message> {
    container(
        column![
            text("Battery Info").size(20),
            text(format!("Voltage: {:.1} V", data.voltage)),
            text(format!("Current: {:.1} A", data.current)),
            text(format!("Charge: {:.1} %", data.charge)),
            text(format!("Temp: {:.1} °C", data.temp))
        ]
        .spacing(5)
        .align_items(Alignment::Start),
    )
    .padding(10)
    .width(Length::FillPortion(1))
    .style(iced::theme::Container::Box)
    .into()
}
