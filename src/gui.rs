use iced::{executor, Application, Command, Element, Settings, Theme};

pub struct TelemetryGui;

impl Application for TelemetryGui {
    type Executor = executor::Tokio;
    type Message = ();
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (TelemetryGui, Command::none())
    }

    fn title(&self) -> String {
        String::from("Telemetry RS")
    }

    fn update(&mut self, _message: Self::Message) -> Command<Self::Message> {
        Command::none()
    }

    fn view(&self) -> Element<Self::Message> {
        "Telemetry GUI Placeholder".into()
    }
}

pub fn run_gui() -> iced::Result {
    TelemetryGui::run(Settings::default())
}
