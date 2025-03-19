use iced::widget::Text;
use iced::{Element, Sandbox};

pub struct TelemetryGui;

#[derive(Debug, Clone)]
pub enum Message {}

impl Sandbox for TelemetryGui {
    type Message = Message;

    fn new() -> Self {
        Self
    }

    fn title(&self) -> String {
        String::from("Telemetry RS - GUI")
    }

    fn update(&mut self, message: Message) {
        match message {
            // messages handling logic will go here later
        }
    }

    fn view(&self) -> Element<Message> {
        Text::new("Hello Telemetry GUI!").into()
    }
}
