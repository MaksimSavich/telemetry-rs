use crate::can::CanDecoder;
use crate::serial::SerialManager;
use iced::{subscription, Application, Command, Element, Subscription, Theme};
use socketcan::{CanFrame, CanSocket, EmbeddedFrame, Socket};
use std::collections::HashMap;
use std::sync::Arc;

// Import our component modules
mod gui_modules;
use crate::gui_modules::*;

pub struct TelemetryGui {
    speed_mph: f64,
    direction: String,
    bps_state: String,
    battery_voltage: f64,
    battery_current: f64,
    battery_charge: f64,
    battery_temp: f64,
    bps_ontime: u64,
    latest_fault: Option<String>,
    fullscreen: bool,
    active_faults: HashMap<String, Fault>,
    fault_history: Vec<Fault>,
    decoder: CanDecoder,
    theme: Theme,

    // Serial communication
    serial_manager: SerialManager,
    available_ports: Vec<String>,
    selected_port: Option<String>,
    serial_status: String,

    // LoRa enabled flag
    lora_enabled: bool,
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
        let serial_manager = SerialManager::new();
        let available_ports = SerialManager::list_available_ports();

        (
            Self {
                latest_fault: None,
                direction: "Neutral".into(),
                fullscreen: false,
                speed_mph: 0.0,
                battery_voltage: 0.0,
                battery_current: 0.0,
                battery_charge: 0.0,
                battery_temp: 0.0,
                bps_ontime: 0,
                bps_state: "Standby".into(),
                active_faults: HashMap::new(),
                fault_history: Vec::new(),
                theme: iced::Theme::Dark,
                decoder: CanDecoder::new("telemetry.dbc"),

                // Serial management
                serial_manager,
                available_ports,
                selected_port: None,
                serial_status: "Disconnected".into(),
                lora_enabled: false,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Telemetry RS - Responsive GUI".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CanFrameReceived(decoded_str, frame) => {
                // Process telemetry data
                for line in decoded_str.lines() {
                    if let Some((signal, val)) = line.split_once(": ") {
                        match signal {
                            "Actual_Speed_RPM" => self.speed_mph = val.parse().unwrap_or(0.0),
                            "Direction" => self.direction = val.to_string(),
                            "BPS_Voltage_V" => self.battery_voltage = val.parse().unwrap_or(0.0),
                            "BPS_Current_A" => self.battery_current = val.parse().unwrap_or(0.0),
                            "Charge_Level" => self.battery_charge = val.parse().unwrap_or(0.0),
                            "Supp_Temperature_C" => self.battery_temp = val.parse().unwrap_or(0.0),
                            "BPS_ON_Time" => self.bps_ontime = val.parse().unwrap_or(0),
                            "BPS_State" => self.bps_state = val.to_string(),
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
                                                timestamp: chrono::Utc::now(),
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

                // If LoRa transmission is enabled, send the CAN frame
                if self.lora_enabled {
                    // Extract frame data
                    let raw_id = match frame.id() {
                        socketcan::Id::Standard(std_id) => std_id.as_raw() as u32,
                        socketcan::Id::Extended(ext_id) => ext_id.as_raw(),
                    };

                    // Send via serial
                    if let Err(e) = self.serial_manager.send_can_frame(raw_id, frame.data()) {
                        println!("Failed to send CAN frame over serial: {}", e);
                        self.serial_status = format!("Error: {}", e);
                    } else {
                        // Successfully sent
                        self.serial_status = format!("Sent: ID 0x{:X}", raw_id);
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

            Message::PortsRefreshed(ports) => {
                self.available_ports = ports;
            }

            Message::PortSelected(port) => {
                self.selected_port = Some(port);
            }

            Message::ConnectSerialPort => {
                if let Some(port) = &self.selected_port {
                    match self.serial_manager.connect(port, 115200) {
                        Ok(_) => {
                            self.serial_status = format!("Connected to {}", port);
                        }
                        Err(e) => {
                            self.serial_status = format!("Error: {}", e);
                        }
                    }
                }
            }

            Message::ToggleLoRa => {
                self.lora_enabled = !self.lora_enabled;
                self.serial_status = if self.lora_enabled {
                    "LoRa transmission enabled".to_string()
                } else {
                    "LoRa transmission disabled".to_string()
                };
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // Create data structs for each component
        let battery_data = BatteryData {
            voltage: self.battery_voltage,
            current: self.battery_current,
            charge: self.battery_charge,
            temp: self.battery_temp,
        };

        let bps_data = BpsData {
            ontime: self.bps_ontime,
            state: self.bps_state.clone(),
        };

        let status_data = StatusData {
            direction: self.direction.clone(),
            latest_fault: self.latest_fault.clone(),
        };

        let serial_config = SerialConfig {
            available_ports: self.available_ports.clone(),
            selected_port: self.selected_port.clone(),
            serial_status: self.serial_status.clone(),
            lora_enabled: self.lora_enabled,
        };

        // Use our components
        let direction_element = direction_text(&self.direction);
        let speed_element = speed_text(self.speed_mph);
        let status_element = status_box(&status_data);
        let battery_element = battery_box(&battery_data);
        let bps_element = bps_box(&bps_data);
        let serial_element = serial_panel(&serial_config);
        let fault_element = fault_section(&self.active_faults);

        // Use the layout utility to organize everything
        main_layout(
            self.fullscreen,
            direction_element,
            speed_element,
            status_element,
            battery_element,
            bps_element,
            serial_element,
            fault_element,
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        let decoder = self.decoder.clone();
        subscription::unfold("can_subscription", decoder, |decoder| async {
            let socket = CanSocket::open("can0").expect("CAN socket failed");
            socket.set_nonblocking(false).unwrap();
            loop {
                if let Ok(frame) = socket.read_frame() {
                    println!("Received CAN frame: {:?}", frame);
                    if let Some(decoded) = decoder.decode(frame.clone()) {
                        return (Message::CanFrameReceived(decoded, frame), decoder);
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
    }
}
