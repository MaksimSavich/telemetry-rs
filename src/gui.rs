use crate::can::CanDecoder;
use iced::widget::{column, text};
use iced::{subscription, Application, Command, Element, Subscription};
use socketcan::{CanSocket, Socket};

pub struct TelemetryGui {
    latest_frame: String,
    decoder: CanDecoder,
}

#[derive(Debug, Clone)]
pub enum Message {
    CanFrameReceived(String),
}

impl Application for TelemetryGui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                latest_frame: "Waiting for decoded CAN data...".into(),
                decoder: CanDecoder::new("telemetry.dbc"),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Telemetry RS - GUI (Decoded)".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        if let Message::CanFrameReceived(frame_str) = message {
            self.latest_frame = frame_str;
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        column![
            text("Telemetry GUI - Latest Decoded CAN Data:").size(24),
            text(&self.latest_frame).size(20),
        ]
        .padding(20)
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
