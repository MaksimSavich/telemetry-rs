use crate::can::CanDecoder;
use crate::serial::SerialManager;
use iced::{subscription, time, Application, Command, Element, Subscription, Theme};
use socketcan::{CanFrame, CanSocket, EmbeddedFrame, Socket, StandardId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

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

    // Fault panel state
    fault_panel_expanded: bool,
    current_fault_index: usize,

    // Serial status for thread communication
    serial_status_shared: Arc<Mutex<String>>,
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

                // Fault panel state
                fault_panel_expanded: false,
                current_fault_index: 0,

                // Thread communication
                serial_status_shared: Arc::new(Mutex::new("Disconnected".into())),
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
                // Print frame for debugging
                let raw_id = match frame.id() {
                    socketcan::Id::Standard(std_id) => std_id.as_raw() as u32,
                    socketcan::Id::Extended(ext_id) => ext_id.as_raw(),
                };

                println!(
                    "Processing frame: ID=0x{:X}, Data={:?}",
                    raw_id,
                    frame.data()
                );

                // Process telemetry data
                for line in decoded_str.lines() {
                    if let Some((signal, val)) = line.split_once(": ") {
                        println!("  Signal: {}, Value: {}", signal, val);
                        match signal {
                            "Actual_Speed_RPM" => match val.parse::<f64>() {
                                Ok(v) => {
                                    self.speed_mph =
                                        (v * 21.25 * std::f64::consts::PI * 60.0) / 63360.0
                                }
                                Err(e) => println!("Failed to parse speed: {}", e),
                            },
                            "Direction" => self.direction = val.to_string(),
                            "BPS_Voltage_V" => match val.parse::<f64>() {
                                Ok(v) => self.battery_voltage = v,
                                Err(e) => println!("Failed to parse voltage: {}", e),
                            },
                            "BPS_Current_A" => match val.parse::<f64>() {
                                Ok(v) => self.battery_current = v,
                                Err(e) => println!("Failed to parse current: {}", e),
                            },
                            "Charge_Level" => match val.parse::<f64>() {
                                Ok(v) => self.battery_charge = v,
                                Err(e) => println!("Failed to parse charge: {}", e),
                            },
                            "Supp_Temperature_C" => match val.parse::<f64>() {
                                Ok(v) => self.battery_temp = v,
                                Err(e) => println!("Failed to parse temperature: {}", e),
                            },
                            "BPS_ON_Time" => match val.parse::<u64>() {
                                Ok(v) => self.bps_ontime = v,
                                Err(e) => println!("Failed to parse BPS on time: {}", e),
                            },
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
                                        println!("Fault detected: {} = {}", fault_name, val);
                                        if !self.active_faults.contains_key(&fault_name) {
                                            let new_fault = Fault {
                                                name: fault_name.clone(),
                                                timestamp: chrono::Utc::now(),
                                                is_active: true,
                                                value: val.to_owned(),
                                            };
                                            println!("Creating new fault entry");
                                            self.active_faults
                                                .insert(fault_name.clone(), new_fault.clone());
                                            self.fault_history.push(new_fault);
                                            println!("Active faults: {}", self.active_faults.len());
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

                // If LoRa transmission is enabled, send the CAN frame in a non-blocking way
                if self.lora_enabled {
                    // Clone what we need for the thread
                    let serial_manager = self.serial_manager.clone();
                    let frame_data = frame.data().to_vec();
                    let frame_id = raw_id;
                    let status_shared = Arc::clone(&self.serial_status_shared);

                    // Spawn a thread to handle serial sending
                    thread::spawn(move || {
                        if let Err(e) = serial_manager.send_can_frame(frame_id, &frame_data) {
                            println!("Failed to send CAN frame over serial: {}", e);

                            // Update status
                            if let Ok(mut status) = status_shared.lock() {
                                *status = format!("Error: {}", e);
                            }
                        } else {
                            // Successfully sent
                            if let Ok(mut status) = status_shared.lock() {
                                *status = format!("Sent: ID 0x{:X}", frame_id);
                            }
                        }
                    });

                    // Update our status from the shared status
                    if let Ok(status) = self.serial_status_shared.lock() {
                        self.serial_status = status.clone();
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
                self.current_fault_index = 0;
            }

            Message::ToggleFaultPanelExpanded => {
                self.fault_panel_expanded = !self.fault_panel_expanded;
            }

            Message::CycleFault => {
                // Only cycle if we have active faults
                if !self.active_faults.is_empty() {
                    self.current_fault_index =
                        (self.current_fault_index + 1) % self.active_faults.len();
                }
            }

            Message::PortSelected(port) => {
                self.selected_port = Some(port);
            }

            Message::ConnectSerialPort => {
                if let Some(port) = &self.selected_port {
                    match self.serial_manager.connect(port, 115200) {
                        Ok(_) => {
                            let status = format!("Connected to {}", port);
                            self.serial_status = status.clone();
                            if let Ok(mut shared) = self.serial_status_shared.lock() {
                                *shared = status;
                            }
                        }
                        Err(e) => {
                            let status = format!("Error: {}", e);
                            self.serial_status = status.clone();
                            if let Ok(mut shared) = self.serial_status_shared.lock() {
                                *shared = status;
                            }
                        }
                    }
                }
            }

            Message::ToggleLoRa => {
                self.lora_enabled = !self.lora_enabled;
                let status = if self.lora_enabled {
                    "LoRa transmission enabled".to_string()
                } else {
                    "LoRa transmission disabled".to_string()
                };
                self.serial_status = status.clone();
                if let Ok(mut shared) = self.serial_status_shared.lock() {
                    *shared = status;
                }
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

        // Create fault panel with expanded state and current index
        let fault_element = fault_section(
            &self.active_faults,
            self.fault_panel_expanded,
            self.current_fault_index,
        );

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
        // Combine subscriptions
        Subscription::batch(vec![
            // Improved CAN subscription
            {
                let decoder = self.decoder.clone();
                subscription::unfold("can_subscription", decoder, |decoder| async {
                    let socket = match CanSocket::open("can0") {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Failed to open CAN socket: {}", e);
                            // Sleep and try again
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            // Create a dummy frame for the error case
                            let dummy_frame = match CanFrame::new(
                                socketcan::Id::Standard(StandardId::new(0).unwrap()),
                                &[0; 8],
                            ) {
                                Some(frame) => frame,
                                None => {
                                    // If we can't even create a dummy frame, something is very wrong
                                    panic!("Failed to create dummy CAN frame");
                                }
                            };
                            return (
                                Message::CanFrameReceived("CAN Error".to_string(), dummy_frame),
                                decoder,
                            );
                        }
                    };

                    // Set non-blocking mode
                    if let Err(e) = socket.set_nonblocking(true) {
                        eprintln!("Failed to set non-blocking mode: {}", e);
                    }

                    loop {
                        match socket.read_frame() {
                            Ok(frame) => {
                                println!("Received CAN frame: {:?}", frame);
                                // Always pass the frame along, even if decoding fails
                                let decoded = decoder
                                    .decode(frame.clone())
                                    .unwrap_or_else(|| format!("Failed to decode: {:?}", frame));
                                return (Message::CanFrameReceived(decoded, frame), decoder);
                            }
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::WouldBlock {
                                    // No data available, just sleep briefly
                                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                                } else {
                                    eprintln!("CAN read error: {}", e);
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                }
                            }
                        }
                    }
                })
            },
            // Add a timer for cycling through faults when not expanded
            // Only subscribe if there are active faults and panel is not expanded
            if !self.active_faults.is_empty() && !self.fault_panel_expanded {
                time::every(std::time::Duration::from_secs(2)).map(|_| Message::CycleFault)
            } else {
                Subscription::none()
            },
        ])
    }
}
