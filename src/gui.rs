use iced::subscription;
use iced::widget::{column, text};
use iced::{Application, Command, Element, Subscription};
use socketcan::{CanSocket, EmbeddedFrame, Socket};

pub struct TelemetryGui {
    latest_frame: String,
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

    fn new(_flags: ()) -> (Self, iced::Command<Message>) {
        (
            Self {
                latest_frame: "No data yet...".to_string(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Telemetry RS - GUI".to_string()
    }

    fn update(&mut self, message: Message) -> iced::Command<Message> {
        match message {
            Message::CanFrameReceived(frame_str) => {
                self.latest_frame = frame_str;
            }
        }
        iced::Command::none()
    }

    fn view(&self) -> Element<Message> {
        column![
            text("Telemetry GUI - Latest CAN Frame:").size(24),
            text(&self.latest_frame).size(20),
        ]
        .padding(20)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        subscription::unfold("can_subscription", (), |_| async {
            can_frame_listener().await
        })
        .map(Message::CanFrameReceived)
    }
}

// Asynchronous listener to subscribe to CAN frames
async fn can_frame_listener() -> (String, ()) {
    let socket = CanSocket::open("vcan0").expect("Unable to open CAN socket");
    socket.set_nonblocking(false).unwrap();

    match socket.read_frame() {
        Ok(frame) => {
            let raw_id = match frame.id() {
                socketcan::Id::Standard(std_id) => std_id.as_raw() as u32,
                socketcan::Id::Extended(ext_id) => ext_id.as_raw(),
            };

            let data_hex: Vec<String> = frame
                .data()
                .iter()
                .map(|byte| format!("{:02X}", byte))
                .collect();

            let frame_str = format!("ID={:03X} Data=[{}]", raw_id, data_hex.join(" "));

            (frame_str, ())
        }
        Err(_) => ("Error reading CAN frame".to_string(), ()),
    }
}
