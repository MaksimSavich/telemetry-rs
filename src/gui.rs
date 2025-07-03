// Optimized src/gui.rs file with enhanced batching integration

use crate::can::CanDecoder;
use crate::logger::CanLogger;
use crate::serial::SerialManager;
use chrono::Local;
use iced::{subscription, time, Application, Command, Element, Subscription, Theme};
use socketcan::{CanFrame, CanSocket, EmbeddedFrame, Socket, StandardId};
use std::collections::HashMap;

use crate::gui_modules::*;
use rand;

pub struct TelemetryGui {
    // CAN status
    can_connected: bool,

    // Motor data
    motor1_speed_rpm: f64,
    motor2_speed_rpm: f64,
    motor1_direction: String,
    motor2_direction: String,

    speed_mph: f64, // This becomes the calculated result
    direction: String,

    motor1_last_update: Option<std::time::Instant>,
    motor2_last_update: Option<std::time::Instant>,

    // MPPT data
    mppt_data: MpptData,

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
    fault_cycle_timer: u32,    // Timer for cycling (increments every update)
    fault_cycle_interval: u32, // Number of ticks between cycles (3 seconds = 15 ticks at 200ms)

    // System components
    decoder: CanDecoder,
    logger: Option<CanLogger>,
    theme: Theme,
    serial_manager: SerialManager,

    // Radio status
    rfd_connected: bool,

    // Enable/disable flags
    rfd_enabled: bool,

    // Configuration mappings
    gui_value_mappings: HashMap<(&'static str, &'static str), Vec<GuiValueType>>,
    fault_signal_config: HashMap<&'static str, Vec<&'static str>>,
}

impl Application for TelemetryGui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = bool; // rfd_enabled

    fn theme(&self) -> Self::Theme {
        iced::Theme::Dark
    }

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let rfd_enabled = flags;

        // Create enhanced serial manager with improved batching
        let serial_manager = Self::create_enhanced_serial_manager(rfd_enabled);

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

                motor1_speed_rpm: 0.0,
                motor2_speed_rpm: 0.0,
                motor1_direction: "Neutral".into(),
                motor2_direction: "Neutral".into(),
                speed_mph: 0.0,
                motor1_last_update: None,
                motor2_last_update: None,
                battery_voltage: 0.0,
                battery_current: 0.0,
                battery_charge: 0.0,
                battery_temp: 0.0,
                battery_temp_hi: 0.0,
                battery_temp_lo: 0.0,
                bps_ontime: 0,
                bps_state: "Standby".into(),
                active_faults: HashMap::new(),

                // Initialize fault cycling state - faster cycling
                fault_page_index: 0,
                fault_cycle_timer: 0,
                fault_cycle_interval: 15, // 3 seconds at 200ms per tick

                theme: iced::Theme::Dark,
                decoder: CanDecoder::new("telemetry.dbc"),
                logger,
                serial_manager,
                rfd_connected: false,
                rfd_enabled,
                current_time: Local::now().format("%H:%M:%S").to_string(),
                mppt_data: MpptData::default(),

                // Initialize configuration mappings
                gui_value_mappings: get_gui_value_mappings(),
                fault_signal_config: get_fault_signal_config(),
            },
            iced::window::change_mode(iced::window::Id::MAIN, iced::window::Mode::Fullscreen),
        )
    }

    fn title(&self) -> String {
        format!(
            "Telemetry RS - RFD: {}",
            if self.rfd_enabled { "ON" } else { "OFF" }
        )
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CanFrameReceived(decoded_str, frame) => {
                // Mark CAN as connected
                self.can_connected = true;

                // Log the frame (non-blocking)
                if let Some(logger) = &mut self.logger {
                    if let Err(e) = logger.log_frame(&frame) {
                        // Don't print every logging error to avoid console spam
                        if rand::random::<u8>() < 10 {
                            // Print ~4% of errors
                            eprintln!("Failed to log CAN frame: {}", e);
                        }
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
                    0x776 => "BPS_System",
                    0x777 => "BPS_Thing",
                    0x200 | 0x201 => "MPPT1",
                    0x202 | 0x203 => "MPPT2",
                    // Motor Controller 1 (ID ending in 05)
                    id if id == 0x0CF11E05
                        || id == 0x0CF11F05
                        || (id & 0xFFFFFF0F) == 0x0CF11E05
                        || (id & 0xFFFFFF0F) == 0x0CF11F05 =>
                    {
                        "MotorController_1"
                    }
                    // Motor Controller 2 (ID ending in 06)
                    id if id == 0x0CF11E06
                        || id == 0x8CF11F06
                        || (id & 0xFFFFFF0F) == 0x0CF11E06
                        || (id & 0xFFFFFF0F) == 0x0CF11F06 =>
                    {
                        "MotorController_2"
                    }
                    _ => "Unknown",
                };

                // Clear existing DTC faults when processing new DTC message
                if message_name == "BMS_DTC" {
                    self.active_faults
                        .retain(|name, _| !name.starts_with("Fault_DTC"));
                }

                // Process telemetry data using mapping system
                for line in decoded_str.lines() {
                    if let Some((signal, val)) = line.split_once(": ") {
                        // Check if this signal updates a GUI value
                        if let Some(gui_value_types) =
                            self.gui_value_mappings.get(&(message_name, signal))
                        {
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

                        // Handle special DTC fault processing
                        if message_name == "BMS_DTC" {
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
                                }
                            }
                        }
                    }
                }

                // UPDATED: Send the CAN frame using enhanced batching system
                // This now includes automatic filtering and intelligent batching
                self.send_can_frame_to_modems_enhanced(raw_id, frame.data());
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

                // Update modem connection status (enhanced monitoring)
                self.update_modem_status_enhanced();

                // Handle fault cycling (faster)
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
        let radio_status = radio_status_indicators(self.rfd_connected && self.rfd_enabled);
        let mppt_info = mppt_info_box(&self.mppt_data, &bps_data);
        let speed_direction = direction_speed_display(&self.direction, self.speed_mph);
        let battery_info = battery_box(&battery_data);
        let fault_display = fault_display(&self.active_faults, self.fault_page_index);
        let time_display = time_display(&self.current_time);

        // Create warning indicator for high battery current
        let warning_indicator = if self.battery_current > 70.0 {
            Some(battery_current_warning())
        } else {
            None
        };

        // Use the layout utility to organize everything
        main_layout(
            self.fullscreen,
            can_status,
            radio_status,
            mppt_info,
            speed_direction,
            battery_info,
            bps_info,
            fault_display,
            time_display,
            warning_indicator,
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        // Combine subscriptions with optimized intervals
        Subscription::batch(vec![
            // Enhanced CAN subscription with better error handling
            {
                let decoder = self.decoder.clone();
                subscription::unfold("enhanced_can_subscription", decoder, |decoder| async {
                    let socket = match CanSocket::open("can0") {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Failed to open CAN socket: {}", e);
                            // Sleep and try again
                            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
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

                    // Set non-blocking mode with minimal timeout
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
                                    // No data available, sleep very briefly for responsiveness
                                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                                } else {
                                    eprintln!("CAN read error: {}", e);
                                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                }
                            }
                        }
                    }
                })
            },
            // Timer for updating time and checking connections - faster refresh
            time::every(std::time::Duration::from_millis(200)).map(|_| Message::Tick),
        ])
    }
}

impl TelemetryGui {
    // UPDATED: Create SerialManager with enhanced batching
    fn create_enhanced_serial_manager(rfd_enabled: bool) -> SerialManager {
        let mut manager = SerialManager::new();

        // Configure modem settings
        manager.set_rfd_enabled(rfd_enabled);

        // Start background scanning
        if let Err(e) = manager.start_background_scanning() {
            println!("Failed to start background scanning: {}", e);
        }

        // Start enhanced batching
        if let Err(e) = manager.start_batching() {
            println!("Failed to start enhanced batching: {}", e);
        } else {
            println!("✓ Enhanced batching started successfully");
            println!("  - Automatic message filtering enabled");
            println!("  - Synchronization markers enabled");
            println!("  - Error recovery enabled");
        }

        manager
    }

    // UPDATED: Enhanced CAN frame transmission with intelligent batching
    fn send_can_frame_to_modems_enhanced(&self, can_id: u32, data: &[u8]) {
        // The enhanced SerialManager now automatically handles:
        // - Message filtering to prevent spam (especially 0x300)
        // - Intelligent batching with proper synchronization
        // - Error recovery and health monitoring
        // - Rate limiting based on DBC transmission intervals

        if let Err(e) = self.serial_manager.send_can_frame(can_id, data) {
            // Only log errors occasionally to prevent console spam
            if rand::random::<u8>() < 5 {
                // ~2% of errors
                eprintln!("CAN frame transmission error: {}", e);
            }
        }
    }

    // UPDATED: Enhanced modem status monitoring with batching statistics
    fn update_modem_status_enhanced(&mut self) {
        // Check enabled state first
        if self.rfd_enabled {
            if let Ok(rfd_status) = self.serial_manager.rfd_status.try_lock() {
                self.rfd_connected = rfd_status.connected;
            }
        } else {
            self.rfd_connected = false;
        }

        // Monitor batch queue health
        let rfd_queue = self.serial_manager.get_batch_stats();

        // Log enhanced statistics periodically for monitoring
        static mut LAST_STATS_LOG: Option<std::time::Instant> = None;
        unsafe {
            let now = std::time::Instant::now();
            let should_log = match LAST_STATS_LOG {
                Some(last) => now.duration_since(last).as_secs() >= 30, // Every 30 seconds
                None => true,
            };

            if should_log {
                if rfd_queue > 0 {
                    println!("📊 Batch queues - RFD: {}", rfd_queue);
                }

                // Check for queue buildup (potential issue)
                if rfd_queue > 20 {
                    println!("⚠ Large batch queues detected - possible transmission issues");
                }

                LAST_STATS_LOG = Some(now);
            }
        }
    }

    // Helper method to update GUI values based on the configuration
    fn update_gui_value(&mut self, gui_value_type: &GuiValueType, value: &str) {
        match gui_value_type {
            GuiValueType::Motor1Speed => {
                if let Ok(v) = value.parse::<f64>() {
                    self.motor1_speed_rpm = v;
                    self.motor1_last_update = Some(std::time::Instant::now());
                    // Trigger speed recalculation immediately
                    self.update_vehicle_speed();
                }
            }
            GuiValueType::Motor2Speed => {
                if let Ok(v) = value.parse::<f64>() {
                    self.motor2_speed_rpm = v;
                    self.motor2_last_update = Some(std::time::Instant::now());
                    // Trigger speed recalculation immediately
                    self.update_vehicle_speed();
                }
            }
            GuiValueType::Motor1Direction => {
                self.motor1_direction = value.to_string();
                self.update_vehicle_direction();
            }
            GuiValueType::Motor2Direction => {
                self.motor2_direction = value.to_string();
                self.update_vehicle_direction();
            }
            GuiValueType::Mppt1InputVoltage => {
                if let Ok(v) = value.parse::<f64>() {
                    self.mppt_data.mppt1_input_voltage = v;
                }
            }
            GuiValueType::Mppt1InputCurrent => {
                if let Ok(v) = value.parse::<f64>() {
                    self.mppt_data.mppt1_input_current = v;
                }
            }
            GuiValueType::Mppt1OutputVoltage => {
                if let Ok(v) = value.parse::<f64>() {
                    self.mppt_data.mppt1_output_voltage = v;
                }
            }
            GuiValueType::Mppt1OutputCurrent => {
                if let Ok(v) = value.parse::<f64>() {
                    self.mppt_data.mppt1_output_current = v;
                }
            }
            GuiValueType::Mppt2InputVoltage => {
                if let Ok(v) = value.parse::<f64>() {
                    self.mppt_data.mppt2_input_voltage = v;
                }
            }
            GuiValueType::Mppt2InputCurrent => {
                if let Ok(v) = value.parse::<f64>() {
                    self.mppt_data.mppt2_input_current = v;
                }
            }
            GuiValueType::Mppt2OutputVoltage => {
                if let Ok(v) = value.parse::<f64>() {
                    self.mppt_data.mppt2_output_voltage = v;
                }
            }
            GuiValueType::Mppt2OutputCurrent => {
                if let Ok(v) = value.parse::<f64>() {
                    self.mppt_data.mppt2_output_current = v;
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
            // BMS data handling (keeping existing structure)
            GuiValueType::BmsPackDcl => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsPackDclKw => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsPackCcl => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsPackCclKw => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsPackDod => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsPackHealth => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsAdaptiveSoc => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsPackSoc => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsAdaptiveAmphours => {
                // BMS data no longer displayed in main info box
            }
            GuiValueType::BmsPackAmphours => {
                // BMS data no longer displayed in main info box
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

    fn update_vehicle_speed(&mut self) {
        // Check data freshness (optional - helps with stale data)
        let now = std::time::Instant::now();
        let max_age = std::time::Duration::from_millis(500); // 500ms max age

        let motor1_fresh = self
            .motor1_last_update
            .map(|t| now.duration_since(t) < max_age)
            .unwrap_or(false);
        let motor2_fresh = self
            .motor2_last_update
            .map(|t| now.duration_since(t) < max_age)
            .unwrap_or(false);

        // Calculate speed using available data
        let (motor1_rpm, motor2_rpm) = match (motor1_fresh, motor2_fresh) {
            (true, true) => (self.motor1_speed_rpm, self.motor2_speed_rpm),
            (true, false) => (self.motor1_speed_rpm, 0.0), // Only motor 1 data is fresh
            (false, true) => (0.0, self.motor2_speed_rpm), // Only motor 2 data is fresh
            (false, false) => (0.0, 0.0),                  // No fresh data
        };

        // Apply your enhanced speed calculation
        self.speed_mph = self.calculate_dual_motor_speed(motor1_rpm, motor2_rpm);
    }

    fn calculate_dual_motor_speed(&self, motor1_rpm: f64, motor2_rpm: f64) -> f64 {
        let min_threshold = 10.0; // Filter noise below 10 RPM
        let wheel_diameter = 23.5; // Make this configurable later

        // Filter out noise
        let motor1_filtered = if motor1_rpm.abs() >= min_threshold {
            motor1_rpm
        } else {
            0.0
        };
        let motor2_filtered = if motor2_rpm.abs() >= min_threshold {
            motor2_rpm
        } else {
            0.0
        };

        // Calculate average RPM
        let avg_rpm = if motor1_filtered != 0.0 && motor2_filtered != 0.0 {
            (motor1_filtered + motor2_filtered) / 2.0
        } else if motor1_filtered != 0.0 {
            motor1_filtered // Only motor 1 active
        } else if motor2_filtered != 0.0 {
            motor2_filtered // Only motor 2 active
        } else {
            0.0 // No motors active
        };

        // Convert to MPH
        let wheel_circumference = wheel_diameter * std::f64::consts::PI;
        (avg_rpm * wheel_circumference * 60.0) / 63360.0
    }

    fn update_vehicle_direction(&mut self) {
        self.direction = match (
            self.motor1_direction.as_str(),
            self.motor2_direction.as_str(),
        ) {
            ("Forward", "Forward") => "Forward".to_string(),
            ("Backward", "Backward") => "Backward".to_string(),
            ("Neutral", "Neutral") => "Neutral".to_string(),
            ("Forward", "Neutral") | ("Neutral", "Forward") => "Forward".to_string(),
            ("Backward", "Neutral") | ("Neutral", "Backward") => "Backward".to_string(),
            ("Forward", "Backward") | ("Backward", "Forward") => "Turning".to_string(),
            _ => "Mixed".to_string(),
        };
    }

    // OPTIONAL: Debugging and monitoring methods
    pub fn enable_batching(&self, enabled: bool) {
        self.serial_manager.enable_batching(enabled);
        println!("Batching {}", if enabled { "enabled" } else { "disabled" });
    }

    pub fn get_batching_stats(&self) -> usize {
        self.serial_manager.get_batch_stats()
    }

    // Get detailed transmission health information
    pub fn get_transmission_health(&self) -> TransmissionHealth {
        let rfd_status = self.serial_manager.rfd_status.lock().unwrap();

        TransmissionHealth {
            rfd_connected: rfd_status.connected,
            rfd_failures: rfd_status.consecutive_failures,
            rfd_last_success: rfd_status.last_success,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransmissionHealth {
    pub rfd_connected: bool,
    pub rfd_failures: u32,
    pub rfd_last_success: Option<std::time::Instant>,
}
