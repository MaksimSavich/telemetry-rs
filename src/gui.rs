// Complete updated src/gui.rs file

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
    battery_temp_lo: f64,
    battery_temp_hi: f64,

    // BPS data
    bps_state: String,
    bps_ontime: u64,

    // UI state
    fullscreen: bool,
    current_time: String,

    // Fault tracking
    active_faults: HashMap<String, Fault>,

    // Fault cycling state
    fault_page_index: usize,   // Current fault page (0-based)
    fault_cycle_timer: u32,    // Timer for cycling (increments every 500ms)
    fault_cycle_interval: u32, // Number of ticks between cycles (6 seconds = 12 ticks at 500ms)

    // System components
    decoder: CanDecoder,
    logger: Option<CanLogger>,
    theme: Theme,
    serial_manager: SerialManager,

    // Radio status
    lora_connected: bool,
    rfd_connected: bool,

    // Enable/disable flags
    lora_enabled: bool,
    rfd_enabled: bool,

    // Configuration mappings
    gui_value_mappings: HashMap<(&'static str, &'static str), Vec<GuiValueType>>,
    fault_signal_config: HashMap<&'static str, Vec<&'static str>>,
}

impl Application for TelemetryGui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = (bool, bool); // (lora_enabled, rfd_enabled)

    fn theme(&self) -> Self::Theme {
        iced::Theme::Dark
    }

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let (lora_enabled, rfd_enabled) = flags;

        let mut serial_manager = SerialManager::new();

        // Set enable/disable flags
        serial_manager.set_lora_enabled(lora_enabled);
        serial_manager.set_rfd_enabled(rfd_enabled);

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
                battery_temp_hi: 0.0,
                battery_temp_lo: 0.0,
                bps_ontime: 0,
                bps_state: "Standby".into(),
                active_faults: HashMap::new(),

                // Initialize fault cycling state
                fault_page_index: 0,
                fault_cycle_timer: 0,
                fault_cycle_interval: 12, // 6 seconds at 500ms per tick

                theme: iced::Theme::Dark,
                decoder: CanDecoder::new("telemetry.dbc"),
                logger,
                serial_manager,
                lora_connected: false,
                rfd_connected: false,
                lora_enabled,
                rfd_enabled,
                current_time: Local::now().format("%H:%M:%S").to_string(),
                bms_data: BmsData::default(),

                // Initialize configuration mappings
                gui_value_mappings: get_gui_value_mappings(),
                fault_signal_config: get_fault_signal_config(),
            },
            iced::window::change_mode(iced::window::Id::MAIN, iced::window::Mode::Fullscreen),
        )
    }

    fn title(&self) -> String {
        format!(
            "Telemetry RS - LoRa: {} | RFD: {}",
            if self.lora_enabled { "ON" } else { "OFF" },
            if self.rfd_enabled { "ON" } else { "OFF" }
        )
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
                    0x310 => "BMS_Limits",
                    0x320 => "BMS_Power",
                    0x330 => "BMS_State",
                    0x340 => "BMS_Capacity",
                    0x360 => "BMS_Temperature",
                    0x776 | 0x777 => "BPS_System",
                    0x0 | 0x1 => "MPPT",
                    // Motor Controller 1 (ID ending in 05)
                    id if id == 0x8CF11E05
                        || id == 0x8CF11F05
                        || (id & 0xFFFFFF0F) == 0x8CF11E05
                        || (id & 0xFFFFFF0F) == 0x8CF11F05 =>
                    {
                        "MotorController_1"
                    }
                    // Motor Controller 2 (ID ending in 06)
                    id if id == 0x8CF11E06
                        || id == 0x8CF11F06
                        || (id & 0xFFFFFF0F) == 0x8CF11E06
                        || (id & 0xFFFFFF0F) == 0x8CF11F06 =>
                    {
                        "MotorController_2"
                    }
                    _ => "Unknown",
                };

                // Process telemetry data using new mapping system
                for line in decoded_str.lines() {
                    if let Some((signal, val)) = line.split_once(": ") {
                        // Check if this signal updates a GUI value
                        if let Some(gui_value_types) =
                            self.gui_value_mappings.get(&(message_name, signal))
                        {
                            // Clone the gui_value_types to avoid borrowing self
                            let gui_value_types_cloned = gui_value_types.clone();
                            for gui_value_type in gui_value_types_cloned {
                                self.update_gui_value(&gui_value_type, val);
                            }
                        }

                        // Check if this signal is configured as a fault signal
                        if let Some(fault_signals) = self.fault_signal_config.get(message_name) {
                            if fault_signals.contains(&signal) {
                                self.process_regular_fault(message_name, signal, val);
                            }
                        }

                        // Handle special DTC fault processing (existing logic)
                        if message_name == "BMS_DTC" {
                            // DTC faults are already handled in the CAN decoder
                            // The decoded_str will contain the fault descriptions
                            if signal.starts_with("Fault_DTC") {
                                let fault_name = signal.to_string();
                                if !val.trim().is_empty()
                                    && val.trim() != "0"
                                    && val.trim() != "0.0"
                                {
                                    // DTC fault is active
                                    let new_fault = Fault {
                                        name: fault_name.clone(),
                                        timestamp: chrono::Utc::now(),
                                        is_active: true,
                                        value: val.to_owned(),
                                        message_name: message_name.to_string(),
                                    };
                                    self.active_faults.insert(fault_name.clone(), new_fault);
                                } else {
                                    // DTC fault is cleared
                                    self.active_faults.remove(&fault_name);
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

                // Handle fault cycling
                let fault_count = self.active_faults.len();
                if fault_count > 5 {
                    // Increment the fault cycle timer
                    self.fault_cycle_timer += 1;

                    // Check if it's time to cycle to the next page
                    if self.fault_cycle_timer >= self.fault_cycle_interval {
                        self.fault_cycle_timer = 0; // Reset timer

                        // Calculate total pages
                        let total_pages = (fault_count + 4) / 5; // Ceiling division for 5 faults per page

                        // Move to next page, wrapping around if necessary
                        self.fault_page_index = (self.fault_page_index + 1) % total_pages;
                    }
                } else {
                    // Reset cycling state when we have 5 or fewer faults
                    self.fault_page_index = 0;
                    self.fault_cycle_timer = 0;
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
            temp_lo: self.battery_temp_lo,
            temp_hi: self.battery_temp_hi,
        };

        let bps_data = BpsData {
            ontime: self.bps_ontime,
            state: self.bps_state.clone(),
        };

        // Create UI elements
        let can_status = can_status_indicator(self.can_connected);
        let radio_status = radio_status_indicators(
            self.lora_connected && self.lora_enabled,
            self.rfd_connected && self.rfd_enabled,
        );
        let bms_info = bms_info_box(&self.bms_data);
        let speed_direction = direction_speed_display(&self.direction, self.speed_mph);
        let battery_info = battery_box(&battery_data);
        let bps_info = bps_box(&bps_data);
        let fault_display = fault_display(&self.active_faults, self.fault_page_index);
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
    // Helper method to update GUI values based on the configuration
    fn update_gui_value(&mut self, gui_value_type: &GuiValueType, value: &str) {
        match gui_value_type {
            GuiValueType::Speed => {
                if let Ok(v) = value.parse::<f64>() {
                    // Convert RPM to MPH: RPM * wheel_circumference * 60 / 63360
                    // wheel_circumference = 23.5" * π
                    self.speed_mph = (v * 23.5 * std::f64::consts::PI * 60.0) / 63360.0;
                }
            }
            GuiValueType::Direction => {
                self.direction = value.to_string();
            }
            GuiValueType::BmsPackDcl => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.pack_dcl = v;
                }
            }
            GuiValueType::BmsPackDclKw => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.pack_dcl_kw = v;
                }
            }
            GuiValueType::BmsPackCcl => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.pack_ccl = v;
                }
            }
            GuiValueType::BmsPackCclKw => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.pack_ccl_kw = v;
                }
            }
            GuiValueType::BmsPackDod => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.pack_dod = v;
                }
            }
            GuiValueType::BmsPackHealth => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.pack_health = v;
                }
            }
            GuiValueType::BmsAdaptiveSoc => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.adaptive_soc = v;
                }
            }
            GuiValueType::BmsPackSoc => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.pack_soc = v;
                }
            }
            GuiValueType::BmsAdaptiveAmphours => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.adaptive_amphours = v;
                }
            }
            GuiValueType::BmsPackAmphours => {
                if let Ok(v) = value.parse::<f64>() {
                    self.bms_data.pack_amphours = v;
                }
            }
            GuiValueType::BatteryVoltage => {
                if let Ok(v) = value.parse::<f64>() {
                    self.battery_voltage = v;
                }
            }
            GuiValueType::BatteryCurrent => {
                if let Ok(v) = value.parse::<f64>() {
                    self.battery_current = v;
                }
            }
            GuiValueType::BatteryCharge => {
                if let Ok(v) = value.parse::<f64>() {
                    self.battery_charge = v;
                }
            }
            GuiValueType::BatteryTemp => {
                if let Ok(v) = value.parse::<f64>() {
                    self.battery_temp = v;
                }
            }
            GuiValueType::BatteryTempLo => {
                if let Ok(v) = value.parse::<f64>() {
                    self.battery_temp_lo = v;
                }
            }
            GuiValueType::BatteryTempHi => {
                if let Ok(v) = value.parse::<f64>() {
                    self.battery_temp_hi = v;
                }
            }
            GuiValueType::BpsOnTime => {
                if let Ok(v) = value.parse::<u64>() {
                    self.bps_ontime = v;
                }
            }
            GuiValueType::BpsState => {
                self.bps_state = value.to_string();
            }
        }
    }

    // Helper method to process regular faults (non-DTC)
    fn process_regular_fault(&mut self, message_name: &str, signal_name: &str, value: &str) {
        let fault_key = format!("{}_{}", message_name, signal_name);

        if is_fault_value(value) {
            // Fault is active
            let new_fault = Fault {
                name: signal_name.to_string(),
                timestamp: chrono::Utc::now(),
                is_active: true,
                value: value.to_owned(),
                message_name: message_name.to_string(),
            };
            self.active_faults.insert(fault_key, new_fault);
        } else {
            // Fault is cleared
            self.active_faults.remove(&fault_key);
        }
    }

    // Helper method to update modem status from the SerialManager
    fn update_modem_status(&mut self) {
        // Only check status for enabled modems
        if self.lora_enabled {
            if let Ok(lora_status) = self.serial_manager.lora_status.lock() {
                self.lora_connected = lora_status.connected;
            }
        } else {
            self.lora_connected = false;
        }

        if self.rfd_enabled {
            if let Ok(rfd_status) = self.serial_manager.rfd_status.lock() {
                self.rfd_connected = rfd_status.connected;
            }
        } else {
            self.rfd_connected = false;
        }
    }

    // Helper method to send CAN frame to enabled modems
    fn send_can_frame_to_modems(&self, can_id: u32, data: &[u8]) {
        // Only send to enabled modems
        if (self.lora_enabled && self.lora_connected) || (self.rfd_enabled && self.rfd_connected) {
            if let Err(e) = self.serial_manager.send_can_frame(can_id, data) {
                // Don't print errors for every frame to avoid flooding console
                // The SerialManager already handles retries and connection status
                eprintln!("Failed to send CAN frame to modems: {}", e);
            }
        }
    }
}
