use crate::can::CanDecoder;
use crate::logger::CanLogger;
use crate::serial::SerialManager;
use chrono::Local;
use iced::{subscription, time, Application, Command, Element, Subscription, Theme};
use socketcan::{CanFrame, CanSocket, EmbeddedFrame, Socket, StandardId};
use std::collections::HashMap;

use crate::gui_modules::*;

pub struct TelemetryGui {
    // CAN status
    can_connected: bool,

    // Motor data
    speed_mph: f64,
    direction: String,

    // BMS data
    bms_data: BmsData,

    // Battery data
    battery_voltage: f64,
    battery_current: f64,
    battery_charge: f64,
    battery_temp: f64,

    // BPS data
    bps_state: String,
    bps_ontime: u64,

    // UI state
    fullscreen: bool,
    current_time: String,

    // Fault tracking
    active_faults: HashMap<String, Fault>,

    // System components
    decoder: CanDecoder,
    logger: Option<CanLogger>,
    theme: Theme,
    serial_manager: SerialManager,

    // Radio status
    lora_connected: bool,
    rfd_connected: bool,
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

        // Start the background scanning for modems
        let mut sm = serial_manager.clone();
        if let Err(e) = sm.start_background_scanning() {
            println!("Failed to start background scanning: {}", e);
        }

        // Initialize logger
        let logger = match CanLogger::new() {
            Ok(logger) => {
                println!("CAN logging started: {:?}", logger.get_log_path());
                Some(logger)
            }
            Err(e) => {
                eprintln!("Failed to initialize CAN logger: {}", e);
                None
            }
        };

        (
            Self {
                can_connected: false,
                direction: "Neutral".into(),
                fullscreen: true,
                speed_mph: 0.0,
                battery_voltage: 0.0,
                battery_current: 0.0,
                battery_charge: 0.0,
                battery_temp: 0.0,
                bps_ontime: 0,
                bps_state: "Standby".into(),
                active_faults: HashMap::new(),
                theme: iced::Theme::Dark,
                decoder: CanDecoder::new("telemetry.dbc"),
                logger,
                serial_manager,
                lora_connected: false,
                rfd_connected: false,
                current_time: Local::now().format("%H:%M:%S").to_string(),
                bms_data: BmsData::default(),
            },
            iced::window::change_mode(iced::window::Id::MAIN, iced::window::Mode::Fullscreen),
        )
    }

    fn title(&self) -> String {
        "Telemetry RS".into()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CanFrameReceived(decoded_str, frame) => {
                // Mark CAN as connected
                self.can_connected = true;

                // Log the frame
                if let Some(logger) = &mut self.logger {
                    if let Err(e) = logger.log_frame(&frame) {
                        eprintln!("Failed to log CAN frame: {}", e);
                    }
                }

                // Get frame ID for fault tracking
                let raw_id = match frame.id() {
                    socketcan::Id::Standard(std_id) => std_id.as_raw() as u32,
                    socketcan::Id::Extended(ext_id) => ext_id.as_raw(),
                };

                // Determine message name from ID
                let message_name = match raw_id {
                    0x300 => "BMS_DTC",
                    0xA0 => "BMS_Limits",
                    0xB1 => "BMS_Power",
                    0xC2 => "BMS_State",
                    0xD3 => "BMS_Capacity",
                    0x776 | 0x777 => "BPS_System",
                    id if id == 0x8CF11E05 || (id & 0xFFFFFF00) == 0x8CF11E00 => {
                        "MotorController_1"
                    }
                    id if id == 0x8CF11F05 || (id & 0xFFFFFF00) == 0x8CF11F00 => {
                        "MotorController_2"
                    }
                    _ => "Unknown",
                };

                // Process telemetry data
                for line in decoded_str.lines() {
                    if let Some((signal, val)) = line.split_once(": ") {
                        match signal {
                            // Motor data
                            "Actual_Speed_RPM" => match val.parse::<f64>() {
                                Ok(v) => {
                                    self.speed_mph =
                                        (v * 21.25 * std::f64::consts::PI * 60.0) / 63360.0
                                }
                                Err(_) => {}
                            },
                            "Direction" => self.direction = val.to_string(),

                            // BMS data
                            "Pack_DCL" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.pack_dcl = v,
                                Err(_) => {}
                            },
                            "Pack_DCL_KW" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.pack_dcl_kw = v,
                                Err(_) => {}
                            },
                            "Pack_CCL" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.pack_ccl = v,
                                Err(_) => {}
                            },
                            "Pack_CCL_KW" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.pack_ccl_kw = v,
                                Err(_) => {}
                            },
                            "Pack_DOD" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.pack_dod = v,
                                Err(_) => {}
                            },
                            "Pack_Health" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.pack_health = v,
                                Err(_) => {}
                            },
                            "Adaptive_SOC" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.adaptive_soc = v,
                                Err(_) => {}
                            },
                            "Pack_SOC" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.pack_soc = v,
                                Err(_) => {}
                            },
                            "Adaptive_Amphours" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.adaptive_amphours = v,
                                Err(_) => {}
                            },
                            "Pack_Amphours" => match val.parse::<f64>() {
                                Ok(v) => self.bms_data.pack_amphours = v,
                                Err(_) => {}
                            },

                            // Battery/BPS data
                            "BPS_Voltage_V" => match val.parse::<f64>() {
                                Ok(v) => self.battery_voltage = v,
                                Err(_) => {}
                            },
                            "BPS_Current_A" => match val.parse::<f64>() {
                                Ok(v) => self.battery_current = v,
                                Err(_) => {}
                            },
                            "Charge_Level" => match val.parse::<f64>() {
                                Ok(v) => self.battery_charge = v,
                                Err(_) => {}
                            },
                            "Supp_Temperature_C" => match val.parse::<f64>() {
                                Ok(v) => self.battery_temp = v,
                                Err(_) => {}
                            },
                            "BPS_ON_Time" => match val.parse::<u64>() {
                                Ok(v) => self.bps_ontime = v,
                                Err(_) => {}
                            },
                            "BPS_State" => self.bps_state = val.to_string(),

                            _ => {
                                // Check for fault signals
                                if signal.starts_with("Fault_") {
                                    let fault_name = signal.to_string();
                                    if !val.trim().is_empty()
                                        && val.trim() != "0"
                                        && val.trim() != "0.0"
                                        && val.trim() != "OK"
                                    {
                                        // Fault is active
                                        let new_fault = Fault {
                                            name: fault_name.clone(),
                                            timestamp: chrono::Utc::now(),
                                            is_active: true,
                                            value: val.to_owned(),
                                            message_name: message_name.to_string(),
                                        };
                                        self.active_faults.insert(fault_name.clone(), new_fault);
                                    } else {
                                        // Fault is cleared
                                        self.active_faults.remove(&fault_name);
                                    }
                                }
                            }
                        }
                    }
                }

                // Send the CAN frame to all enabled modems
                self.send_can_frame_to_modems(raw_id, frame.data());
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

            Message::Tick => {
                // Update current time
                self.current_time = Local::now().format("%H:%M:%S").to_string();

                // Update modem connection status
                self.update_modem_status();
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

        // Create UI elements
        let can_status = can_status_indicator(self.can_connected);
        let radio_status = radio_status_indicators(self.lora_connected, self.rfd_connected);
        let bms_info = bms_info_box(&self.bms_data);
        let speed_direction = direction_speed_display(&self.direction, self.speed_mph);
        let battery_info = battery_box(&battery_data);
        let bps_info = bps_box(&bps_data);
        let fault_display = fault_display(&self.active_faults);
        let time_display = time_display(&self.current_time);

        // Use the layout utility to organize everything
        main_layout(
            self.fullscreen,
            can_status,
            radio_status,
            bms_info,
            speed_direction,
            battery_info,
            bps_info,
            fault_display,
            time_display,
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        // Combine subscriptions
        Subscription::batch(vec![
            // CAN subscription
            {
                let decoder = self.decoder.clone();
                subscription::unfold("can_subscription", decoder, |decoder| async {
                    let socket = match CanSocket::open("vcan0") {
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
                                // Always pass the frame along, even if decoding fails
                                let decoded = decoder
                                    .decode(frame.clone())
                                    .unwrap_or_else(|| format!("Unknown frame: {:?}", frame));
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
            // Timer for updating time and checking connections
            time::every(std::time::Duration::from_millis(500)).map(|_| Message::Tick),
        ])
    }
}

impl TelemetryGui {
    // Helper method to update modem status from the SerialManager
    fn update_modem_status(&mut self) {
        // Update LoRa modem status
        if let Ok(lora_status) = self.serial_manager.lora_status.lock() {
            self.lora_connected = lora_status.connected;
        }

        // Update RFD modem status
        if let Ok(rfd_status) = self.serial_manager.rfd_status.lock() {
            self.rfd_connected = rfd_status.connected;
        }
    }

    // Helper method to send CAN frame to enabled modems
    fn send_can_frame_to_modems(&self, can_id: u32, data: &[u8]) {
        // Send to all connected modems (SerialManager handles the enabled check)
        if let Err(e) = self.serial_manager.send_can_frame(can_id, data) {
            // Don't print errors for every frame to avoid flooding console
            // The SerialManager already handles retries and connection status
        }
    }
}
