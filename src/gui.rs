use crate::can::CanDecoder;
use iced::widget::{column, text};
use iced::{subscription, Application, Command, Element, Subscription};
use socketcan::{CanSocket, Socket};

pub struct TelemetryGui {
    speed_mph: f64,
    direction: String,
    battery_voltage: f64,
    battery_current: f64,
    battery_charge: f64,
    battery_temp: f64,
    latest_fault: Option<String>,
    fullscreen: bool,
    decoder: CanDecoder,
}

#[derive(Debug, Clone)]
pub enum Message {
    CanFrameReceived(String),
    ToggleFullscreen,
}

impl Application for TelemetryGui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

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
                decoder: CanDecoder::new("telemetry.dbc"),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Telemetry RS - GUI (Decoded)".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CanFrameReceived(decoded_str) => {
                for line in decoded_str.lines() {
                    if let Some((signal, val)) = line.split_once(": ") {
                        match signal {
                            "Vehicle_Speed" => self.speed_mph = val.parse().unwrap_or(0.0),
                            "Direction" => self.direction = val.to_string(),
                            "Voltage" => self.battery_voltage = val.parse().unwrap_or(0.0),
                            "Current" => self.battery_current = val.parse().unwrap_or(0.0),
                            "Charge_Level" => self.battery_charge = val.parse().unwrap_or(0.0),
                            "Temp" => self.battery_temp = val.parse().unwrap_or(0.0),
                            "Fault_Active" => self.latest_fault = Some(val.to_string()),
                            _ => {}
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
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let direction = &self.direction;

        let fault_display = self.latest_fault.clone().unwrap_or("No Faults".into());

        let fullscreen_button = iced::widget::button(text(if self.fullscreen {
            "Exit Fullscreen"
        } else {
            "Fullscreen"
        }))
        .on_press(Message::ToggleFullscreen);

        column![
            iced::widget::container(fullscreen_button).align_x(iced::alignment::Horizontal::Right),
            text(direction).size(28),
            text(format!("{:.1} MPH", self.speed_mph)).size(60),
            iced::widget::row![
                column![
                    text("CAN Status").size(20),
                    text(format!("Direction: {}", direction)),
                ]
                .spacing(5),
                column![
                    text("Battery Info").size(20),
                    text(format!("Voltage: {:.1} V", self.battery_voltage)),
                    text(format!("Current: {:.1} A", self.battery_current)),
                    text(format!("Charge: {:.1} %", self.battery_charge)),
                    text(format!("Temp: {:.1} °C", self.battery_temp)),
                ]
                .spacing(5),
            ]
            .spacing(50)
            .padding(20),
            text(format!("Fault: {}", fault_display)).size(18),
            text(format!("{:.1} MPH", self.battery_voltage))
                .size(36)
                .style(iced::Color::WHITE)
        ]
        .padding(20)
        .spacing(10)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let decoder = self.decoder.clone();
        subscription::unfold("can_subscription", decoder, |decoder| async {
            let socket = CanSocket::open("vcan0").expect("CAN socket failed");
            socket.set_nonblocking(false).unwrap();
            loop {
                if let Ok(frame) = socket.read_frame() {
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
