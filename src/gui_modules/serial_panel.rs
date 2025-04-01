use crate::gui_modules::Message;
use iced::widget::{button, column, container, pick_list, row, text};
use iced::{Alignment, Element, Length};

#[derive(Clone)]
pub struct SerialConfig {
    pub available_ports: Vec<String>,
    pub selected_port: Option<String>,
    pub serial_status: String,
    pub lora_enabled: bool,
}

pub fn serial_panel(config: &SerialConfig) -> Element<'static, Message> {
    let port_dropdown = pick_list(
        config.available_ports.clone(), // Clone the vector here
        config.selected_port.clone(),
        Message::PortSelected,
    )
    .width(Length::Fill);

    let connect_button = button("Connect")
        .on_press(Message::ConnectSerialPort)
        .width(Length::Fill);

    let lora_toggle = button(if config.lora_enabled {
        "Disable LoRa Transmission"
    } else {
        "Enable LoRa Transmission"
    })
    .on_press(Message::ToggleLoRa)
    .width(Length::Fill);

    let lora_status = text(format!("Status: {}", config.serial_status));

    container(
        column![
            text("LoRa Configuration").size(20),
            row![
                text("Port:").width(Length::FillPortion(1)),
                port_dropdown.width(Length::FillPortion(3))
            ]
            .spacing(10),
            connect_button,
            lora_toggle,
            lora_status
        ]
        .spacing(10)
        .align_items(Alignment::Start),
    )
    .padding(10)
    .width(Length::Fill)
    .style(iced::theme::Container::Box)
    .into()
}
