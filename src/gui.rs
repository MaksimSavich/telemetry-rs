use crate::can::CanDecoder;
use chrono::{DateTime, Utc};
use iced::widget::container::StyleSheet;
use iced::widget::{button, column, container, row, text};
use iced::{
    subscription, Alignment, Application, Color, Command, Element, Length, Subscription, Theme,
};
use socketcan::{CanSocket, Socket};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Fault {
    name: String,
    timestamp: DateTime<Utc>,
    is_active: bool,
    value: String,
}

pub struct TelemetryGui {
    speed_mph: f64,
    direction: String,
    battery_voltage: f64,
    battery_current: f64,
    battery_charge: f64,
    battery_temp: f64,
    latest_fault: Option<String>,
    fullscreen: bool,
    active_faults: HashMap<String, Fault>,
    fault_history: Vec<Fault>,
    decoder: CanDecoder,
    theme: Theme,
}

#[derive(Debug, Clone)]
pub enum Message {
    CanFrameReceived(String),
    ToggleFullscreen,
    ClearFaults,
}

impl Application for TelemetryGui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn theme(&self) -> Self::Theme {
        iced::Theme::Dark
    }

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                latest_fault: None,
                direction: "Forward".into(),
                fullscreen: false,
                speed_mph: 0.0,
                battery_voltage: 0.0,
                battery_current: 0.0,
                battery_charge: 0.0,
                battery_temp: 0.0,
                active_faults: HashMap::new(),
                fault_history: Vec::new(),
                theme: iced::Theme::Dark,
                decoder: CanDecoder::new("telemetry.dbc"),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Telemetry RS - Responsive GUI".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CanFrameReceived(decoded_str) => {
                for line in decoded_str.lines() {
                    if let Some((signal, val)) = line.split_once(": ") {
                        match signal {
                            "Actual_Speed_RPM" => self.speed_mph = val.parse().unwrap_or(0.0),
                            "Direction" => self.direction = val.to_string(),
                            "Supp_Voltage_V" => self.battery_voltage = val.parse().unwrap_or(0.0),
                            "BPS_Current_A" => self.battery_current = val.parse().unwrap_or(0.0),
                            "Charge_Level" => self.battery_charge = val.parse().unwrap_or(0.0),
                            "Supp_Temperature_C" => self.battery_temp = val.parse().unwrap_or(0.0),
                            _ => {
                                // Check for fault signals
                                if signal.starts_with("Fault_") {
                                    let fault_name = signal.to_string();
                                    if !val.trim().is_empty()
                                        && val.trim() != "0"
                                        && val.trim() != "0.0"
                                    {
                                        // If fault is newly active
                                        println!("Fault detected {}", val);
                                        if !self.active_faults.contains_key(&fault_name) {
                                            let new_fault = Fault {
                                                name: fault_name.clone(),
                                                timestamp: Utc::now(),
                                                is_active: true,
                                                value: val.to_owned(),
                                            };
                                            println!("Creating fault");
                                            self.active_faults
                                                .insert(fault_name.clone(), new_fault.clone());
                                            self.fault_history.push(new_fault);
                                            println!("{}", self.active_faults.len());
                                        }
                                    } else {
                                        // If fault is cleared
                                        if let Some(fault) = self.active_faults.get_mut(&fault_name)
                                        {
                                            fault.is_active = false;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Message::ToggleFullscreen => {
                self.fullscreen = !self.fullscreen;
                return iced::window::change_mode(
                    iced::window::Id::MAIN,
                    if self.fullscreen {
                        iced::window::Mode::Fullscreen
                    } else {
                        iced::window::Mode::Windowed
                    },
                );
            }

            Message::ClearFaults => {
                // Mark all active faults as inactive
                for fault in self.active_faults.values_mut() {
                    fault.is_active = false;
                }
                self.active_faults.clear();
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // Fullscreen button
        let fullscreen_button = container(
            iced::widget::button(text(if self.fullscreen {
                "Exit Fullscreen"
            } else {
                "Fullscreen"
            }))
            .on_press(Message::ToggleFullscreen),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right);

        // Direction text
        let direction_text = container(
            text(&self.direction)
                .size(28)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center);

        // Speed text
        let speed_text = container(
            text(format!("{:.1} MPH", self.speed_mph))
                .size(60)
                .horizontal_alignment(iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center);

        // Status info box
        let status_box = container(
            column![
                text("CAN Status").size(20),
                text(format!("Direction: {}", self.direction)),
                text(format!(
                    "Fault: {}",
                    self.latest_fault.clone().unwrap_or("No Faults".into())
                ))
            ]
            .spacing(5)
            .align_items(Alignment::Start),
        )
        .padding(10)
        .width(Length::FillPortion(1))
        .style(iced::theme::Container::Box);

        // Battery info box
        let battery_box = container(
            column![
                text("Battery Info").size(20),
                text(format!("Voltage: {:.1} V", self.battery_voltage)),
                text(format!("Current: {:.1} A", self.battery_current)),
                text(format!("Charge: {:.1} %", self.battery_charge)),
                text(format!("Temp: {:.1} °C", self.battery_temp))
            ]
            .spacing(5)
            .align_items(Alignment::Start),
        )
        .padding(10)
        .width(Length::FillPortion(1))
        .style(iced::theme::Container::Box);

        // Fault Indicators
        let fault_indicator: iced::widget::Container<'_, Message, Theme, iced::Renderer> =
            container(text("FAULT"))
                .style(if !self.active_faults.is_empty() {
                    println!("Faults detected");
                    iced::theme::Container::Custom(Box::new(move |theme: &Theme| {
                        let mut appearance = theme.appearance(&iced::theme::Container::Box);
                        appearance.background = Some(Color::from_rgb(1.0, 0.0, 0.0).into());
                        appearance.text_color = Some(Color::WHITE);
                        appearance
                    }))
                } else {
                    iced::theme::Container::Box
                })
                .padding(10);

        // Fault List
        let fault_list = column(
            self.active_faults
                .values()
                .map(|fault| text(format!("{}: {} (Active)", fault.name, fault.value)).into())
                .collect::<Vec<_>>(),
        )
        .spacing(5);

        // Clear Faults Button
        let clear_faults_button = button("Clear Faults").on_press(Message::ClearFaults);

        // Fault Section
        let fault_section = column![
            row![fault_indicator, fault_list]
                .spacing(10)
                .align_items(Alignment::Center),
            clear_faults_button
        ]
        .spacing(10);

        // Main layout using columns and rows
        column![
            // Top row for fullscreen button
            fullscreen_button,
            // Direction text
            direction_text,
            // Bottom row for status and battery info
            row![
                status_box, // Left side
                // Speed text
                speed_text,
                battery_box, // Right side
            ],
            // .width(Length::Fill)
            // .spacing(20)
            // .align_items(Alignment::Center)
            fault_section // Add the new fault section here
        ]
        .padding(20)
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_items(Alignment::Center)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let decoder = self.decoder.clone();
        subscription::unfold("can_subscription", decoder, |decoder| async {
            let socket = CanSocket::open("can0").expect("CAN socket failed");
            socket.set_nonblocking(false).unwrap();
            loop {
                if let Ok(frame) = socket.read_frame() {
                    println!("Received CAN frame: {:?}", frame);
                    if let Some(decoded) = decoder.decode(frame) {
                        return (decoded, decoder);
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .map(Message::CanFrameReceived)
    }
}
