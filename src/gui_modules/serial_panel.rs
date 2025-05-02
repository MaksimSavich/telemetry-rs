use crate::gui_modules::Message;
use crate::serial::ModemType;
use iced::widget::{button, column, container, pick_list, row, text};
use iced::{Alignment, Element, Length};

#[derive(Clone)]
pub struct SerialConfig {
    pub available_ports: Vec<String>,

    // LoRa modem configuration
    pub lora_selected_port: Option<String>,
    pub lora_connected: bool,
    pub lora_status: String,
    pub lora_enabled: bool,

    // RFD modem configuration
    pub rfd_selected_port: Option<String>,
    pub rfd_connected: bool,
    pub rfd_status: String,
    pub rfd_enabled: bool,
}

pub fn serial_panel(config: &SerialConfig) -> Element<'static, Message> {
    // Create the LoRa section
    let lora_section = column![
        text("LoRa Configuration").size(20),
        row![
            text("Port:").width(Length::FillPortion(1)),
            pick_list(
                config.available_ports.clone(),
                config.lora_selected_port.clone(),
                |port| Message::PortSelected(ModemType::Lora, port)
            )
            .width(Length::FillPortion(3))
        ]
        .spacing(10),
        button(if config.lora_connected {
            "Disconnect"
        } else {
            "Connect"
        })
        .on_press(if config.lora_connected {
            Message::DisconnectModem(ModemType::Lora)
        } else {
            Message::ConnectSerialPort(ModemType::Lora)
        })
        .width(Length::Fill),
        button(if config.lora_enabled {
            "Disable LoRa Transmission"
        } else {
            "Enable LoRa Transmission"
        })
        .on_press(Message::ToggleModem(ModemType::Lora))
        .width(Length::Fill),
        text(format!("Status: {}", config.lora_status)),
    ]
    .spacing(10)
    .align_items(Alignment::Start)
    .width(Length::FillPortion(1));

    // Create the RFD section
    let rfd_section = column![
        text("RFD 900x2 Configuration").size(20),
        row![
            text("Port:").width(Length::FillPortion(1)),
            pick_list(
                config.available_ports.clone(),
                config.rfd_selected_port.clone(),
                |port| Message::PortSelected(ModemType::Rfd900x, port)
            )
            .width(Length::FillPortion(3))
        ]
        .spacing(10),
        button(if config.rfd_connected {
            "Disconnect"
        } else {
            "Connect"
        })
        .on_press(if config.rfd_connected {
            Message::DisconnectModem(ModemType::Rfd900x)
        } else {
            Message::ConnectSerialPort(ModemType::Rfd900x)
        })
        .width(Length::Fill),
        button(if config.rfd_enabled {
            "Disable RFD Transmission"
        } else {
            "Enable RFD Transmission"
        })
        .on_press(Message::ToggleModem(ModemType::Rfd900x))
        .width(Length::Fill),
        text(format!("Status: {}", config.rfd_status)),
    ]
    .spacing(10)
    .align_items(Alignment::Start)
    .width(Length::FillPortion(1));

    // Combine both sections horizontally
    let combined_sections = row![lora_section, rfd_section]
        .spacing(20)
        .width(Length::Fill);

    container(combined_sections)
        .padding(10)
        .width(Length::Fill)
        .style(iced::theme::Container::Box)
        .into()
}

// Status indicator for modems (to be placed in the header)
pub fn modem_status_indicators(
    lora_connected: bool,
    rfd_connected: bool,
) -> Element<'static, Message> {
    let lora_status = container(
        text(if lora_connected {
            "LoRa: Connected"
        } else {
            "LoRa: Disconnected"
        })
        .size(14),
    )
    .padding(4)
    .style(if lora_connected {
        iced::theme::Container::Custom(Box::new(|theme| {
            let mut appearance = theme.appearance(&iced::theme::Container::Box);
            appearance.background = Some(iced::Color::from_rgb(0.0, 0.8, 0.0).into());
            appearance.text_color = Some(iced::Color::WHITE);
            appearance
        }))
    } else {
        iced::theme::Container::Custom(Box::new(|theme| {
            let mut appearance = theme.appearance(&iced::theme::Container::Box);
            appearance.background = Some(iced::Color::from_rgb(0.8, 0.0, 0.0).into());
            appearance.text_color = Some(iced::Color::WHITE);
            appearance
        }))
    });

    let rfd_status = container(
        text(if rfd_connected {
            "RFD: Connected"
        } else {
            "RFD: Disconnected"
        })
        .size(14),
    )
    .padding(4)
    .style(if rfd_connected {
        iced::theme::Container::Custom(Box::new(|theme| {
            let mut appearance = theme.appearance(&iced::theme::Container::Box);
            appearance.background = Some(iced::Color::from_rgb(0.0, 0.8, 0.0).into());
            appearance.text_color = Some(iced::Color::WHITE);
            appearance
        }))
    } else {
        iced::theme::Container::Custom(Box::new(|theme| {
            let mut appearance = theme.appearance(&iced::theme::Container::Box);
            appearance.background = Some(iced::Color::from_rgb(0.8, 0.0, 0.0).into());
            appearance.text_color = Some(iced::Color::WHITE);
            appearance
        }))
    });

    row![lora_status, rfd_status].spacing(10).into()
}
