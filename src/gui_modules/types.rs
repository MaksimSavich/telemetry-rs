use chrono::{DateTime, Utc};
use iced::{widget::container::StyleSheet, Color, Theme};
use socketcan::CanFrame;
use std::collections::HashMap;

// Re-export common types and messages for all components

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultSeverity {
    Critical = 0, // Highest priority - most severe
    Error = 1,    // Medium priority
    Warning = 2,  // Lowest priority - least severe
}

#[derive(Clone, Debug)]
pub struct Fault {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub is_active: bool,
    pub value: String,
    pub message_name: String, // Added to track which CAN message this came from
    pub severity: FaultSeverity, // New field for severity classification
}

// Message enum shared between all components
#[derive(Debug, Clone)]
pub enum Message {
    CanFrameReceived(String, CanFrame),
    ToggleFullscreen,
    Tick, // For updating time display
}

// Container styling helpers for different fault severities
pub fn create_error_container_style() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(move |theme: &Theme| {
        let mut appearance = theme.appearance(&iced::theme::Container::Box);
        appearance.background = Some(Color::from_rgb(1.0, 0.0, 0.0).into());
        appearance.text_color = Some(Color::WHITE);
        appearance
    }))
}

pub fn create_warning_container_style() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(move |theme: &Theme| {
        let mut appearance = theme.appearance(&iced::theme::Container::Box);
        appearance.background = Some(Color::from_rgb(1.0, 0.8, 0.0).into()); // Yellow background
        appearance.text_color = Some(Color::BLACK); // Black text for better readability on yellow
        appearance
    }))
}

pub fn create_critical_container_style() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(move |theme: &Theme| {
        let mut appearance = theme.appearance(&iced::theme::Container::Box);
        appearance.background = Some(Color::from_rgb(0.8, 0.0, 0.0).into()); // Dark red background
        appearance.text_color = Some(Color::WHITE);
        appearance.border.color = Color::from_rgb(1.0, 0.0, 0.0); // Bright red border
        appearance.border.width = 2.0; // Thicker border for critical
        appearance
    }))
}

// Helper function to get container style based on fault severity
pub fn get_fault_container_style(severity: &FaultSeverity) -> iced::theme::Container {
    match severity {
        FaultSeverity::Warning => create_warning_container_style(),
        FaultSeverity::Error => create_error_container_style(),
        FaultSeverity::Critical => create_critical_container_style(),
    }
}

// DTC fault definitions for BMS with severity classification
pub const DTC_FLAGS_1_FAULTS: &[(u16, &str, FaultSeverity)] = &[
    (0x0001, "Discharge Limit Enforcement", FaultSeverity::Critical),
    (0x0002, "Charger Safety Relay", FaultSeverity::Error),
    (0x0004, "Internal Hardware", FaultSeverity::Critical),
    (0x0008, "Internal Heatsink Thermistor", FaultSeverity::Error),
    (0x0010, "Internal Software", FaultSeverity::Critical),
    (0x0020, "Highest Cell Voltage Too High", FaultSeverity::Critical),
    (0x0040, "Lowest Cell Voltage Too Low", FaultSeverity::Critical),
    (0x0080, "Pack Too Hot", FaultSeverity::Critical),
];

pub const DTC_FLAGS_2_FAULTS: &[(u16, &str, FaultSeverity)] = &[
    (0x0001, "Internal Communication", FaultSeverity::Error),
    (0x0002, "Cell Balancing Stuck Off", FaultSeverity::Warning),
    (0x0004, "Weak Cell", FaultSeverity::Warning),
    (0x0008, "Low Cell Voltage", FaultSeverity::Critical),
    (0x0010, "Open Wiring", FaultSeverity::Critical),
    (0x0020, "Current Sensor", FaultSeverity::Error),
    (0x0040, "Highest Cell Voltage Over 5V", FaultSeverity::Critical),
    (0x0080, "Cell ASIC", FaultSeverity::Critical),
    (0x0100, "Weak Pack", FaultSeverity::Warning),
    (0x0200, "Fan Monitor", FaultSeverity::Warning),
    (0x0400, "Thermistor", FaultSeverity::Error),
    (0x0800, "External Communication", FaultSeverity::Error),
    (0x1000, "Redundant Power Supply", FaultSeverity::Warning),
    (0x2000, "High Voltage Isolation", FaultSeverity::Critical),
    (0x4000, "Input Power Supply", FaultSeverity::Error),
    (0x8000, "Charge Limit Enforcement", FaultSeverity::Critical),
];

// Configuration for GUI value updates using (message_name, signal_name) as key
// Returns a Vec of GuiValueType since one signal might update multiple GUI values
// Returns a Vec of GuiValueType since one signal might update multiple GUI values
pub fn get_gui_value_mappings() -> HashMap<(&'static str, &'static str), Vec<GuiValueType>> {
    let mut mappings = HashMap::new();

    mappings.insert(
        ("MotorController_1", "Actual_Speed_RPM"),
        vec![GuiValueType::Motor1Speed],
    );
    mappings.insert(
        ("MotorController_2", "Actual_Speed_RPM"),
        vec![GuiValueType::Motor2Speed],
    );
    mappings.insert(
        ("MotorController_1", "Status_Of_Command"),
        vec![GuiValueType::Motor1Direction],
    );
    mappings.insert(
        ("MotorController_2", "Status_Of_Command"),
        vec![GuiValueType::Motor2Direction],
    );

    // BMS data
    mappings.insert(("BMS_Limits", "Pack_DCL"), vec![GuiValueType::BmsPackDcl]);
    mappings.insert(
        ("BMS_Limits", "Pack_DCL_KW"),
        vec![GuiValueType::BmsPackDclKw],
    );
    mappings.insert(("BMS_Limits", "Pack_CCL"), vec![GuiValueType::BmsPackCcl]);
    mappings.insert(
        ("BMS_Limits", "Pack_CCL_KW"),
        vec![GuiValueType::BmsPackCclKw],
    );
    mappings.insert(("BMS_State", "Pack_DOD"), vec![GuiValueType::BmsPackDod]);
    mappings.insert(
        ("BMS_State", "Pack_Health"),
        vec![GuiValueType::BmsPackHealth],
    );
    mappings.insert(
        ("BMS_State", "Adaptive_SOC"),
        vec![GuiValueType::BmsAdaptiveSoc],
    );
    mappings.insert(
        ("BMS_State", "Pack_SOC"),
        vec![GuiValueType::BmsPackSoc, GuiValueType::BatteryCharge],
    ); // One signal, two GUI updates
    mappings.insert(
        ("BMS_Capacity", "Adaptive_Amphours"),
        vec![GuiValueType::BmsAdaptiveAmphours],
    );
    mappings.insert(
        ("BMS_Capacity", "Pack_Amphours"),
        vec![GuiValueType::BmsPackAmphours],
    );

    // Battery/BPS data
    mappings.insert(
        ("BMS_Power", "Pack_Inst_Voltage"),
        vec![GuiValueType::BatteryVoltage],
    );
    mappings.insert(
        ("BMS_Power", "Pack_Current"),
        vec![GuiValueType::BatteryCurrent],
    );
    mappings.insert(
        ("BMS_Temperature", "Average_Temperature"),
        vec![GuiValueType::BatteryTemp],
    );
    mappings.insert(
        ("BMS_Temperature", "High_Temperature"),
        vec![GuiValueType::BatteryTempHi],
    );
    mappings.insert(
        ("BMS_Temperature", "Low_Temperature"),
        vec![GuiValueType::BatteryTempLo],
    );
    mappings.insert(("BPS_Thing", "BPS_ON_Time"), vec![GuiValueType::BpsOnTime]);
    mappings.insert(("BPS_Thing", "BPS_State"), vec![GuiValueType::BpsState]);

    // MPPT data - assuming MPPT message structure
    mappings.insert(
        ("MPPT1", "Input_Voltage_V"),
        vec![GuiValueType::Mppt1InputVoltage],
    );
    mappings.insert(
        ("MPPT1", "Input_Current_A"),
        vec![GuiValueType::Mppt1InputCurrent],
    );
    mappings.insert(
        ("MPPT1", "Output_Voltage_V"),
        vec![GuiValueType::Mppt1OutputVoltage],
    );
    mappings.insert(
        ("MPPT1", "Output_Current_A"),
        vec![GuiValueType::Mppt1OutputCurrent],
    );
    mappings.insert(
        ("MPPT2", "Input_Voltage_V"),
        vec![GuiValueType::Mppt2InputVoltage],
    );
    mappings.insert(
        ("MPPT2", "Input_Current_A"),
        vec![GuiValueType::Mppt2InputCurrent],
    );
    mappings.insert(
        ("MPPT2", "Output_Voltage_V"),
        vec![GuiValueType::Mppt2OutputVoltage],
    );
    mappings.insert(
        ("MPPT2", "Output_Current_A"),
        vec![GuiValueType::Mppt2OutputCurrent],
    );

    mappings
}

// Enum for different GUI value types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GuiValueType {
    BmsPackDcl,
    BmsPackDclKw,
    BmsPackCcl,
    BmsPackCclKw,
    BmsPackDod,
    BmsPackHealth,
    BmsAdaptiveSoc,
    BmsPackSoc,
    BmsAdaptiveAmphours,
    BmsPackAmphours,
    BatteryVoltage,
    BatteryCurrent,
    BatteryCharge,
    BatteryTemp,
    BatteryTempHi,
    BatteryTempLo,
    BpsOnTime,
    BpsState,
    Motor1Speed,
    Motor2Speed,
    Motor1Direction,
    Motor2Direction,
    Mppt1InputVoltage,
    Mppt1InputCurrent,
    Mppt1OutputVoltage,
    Mppt1OutputCurrent,
    Mppt2InputVoltage,
    Mppt2InputCurrent,
    Mppt2OutputVoltage,
    Mppt2OutputCurrent,
}

// Configuration for fault signals - defines which signals in which messages are faults
pub fn get_fault_signal_config() -> HashMap<&'static str, Vec<&'static str>> {
    let mut config = HashMap::new();

    // Motor Controller 1 faults
    config.insert(
        "MotorController_1",
        vec![
            "MC_ERR0", "MC_ERR1", "MC_ERR2", "MC_ERR3", "MC_ERR4", "MC_ERR5", "MC_ERR6", "MC_ERR7",
            "MC_ERR8", "MC_ERR9", "MC_ERR10", "MC_ERR11", "MC_ERR12", "MC_ERR13", "MC_ERR14",
            "MC_ERR15",
        ],
    );

    // Motor Controller 2 faults
    config.insert(
        "MotorController_2",
        vec![
            "MC_ERR0", "MC_ERR1", "MC_ERR2", "MC_ERR3", "MC_ERR4", "MC_ERR5", "MC_ERR6", "MC_ERR7",
            "MC_ERR8", "MC_ERR9", "MC_ERR10", "MC_ERR11", "MC_ERR12", "MC_ERR13", "MC_ERR14",
            "MC_ERR15",
        ],
    );

    // MPPT faults
    config.insert("MPPT", vec!["MPPT_Fault"]);

    // BPS System faults
    config.insert(
        "BPS_System",
        vec![
            "Supp_Voltage",
            "Supp_Temperature",
            "DCDC_Voltage",
            "Supp_Charge_Current",
            "Main_Pack_Voltage",
            "BPS_Precharge",
            "BPS_BMS_CAN_Fault",
            "BPS_BMS_CAN_Warning",
            "BPS_BMS_CAN_Timeout",
            "Estop_Fault",
            "Charge_Enabled_Fault",
            "BPS_Faulted_Value",
        ],
    );

    // Note: BMS DTC faults are handled separately via DTC_FLAGS_1_FAULTS and DTC_FLAGS_2_FAULTS
    // They are not included here because they use a different fault detection mechanism

    config
}

// Helper function to check if a signal value indicates a fault (for non-DTC faults)
pub fn is_fault_value(value: &str) -> bool {
    let trimmed = value.trim();
    let upper_value = trimmed.to_uppercase();

    // A signal is NOT a fault if it's empty, "0", "0.0", "OK", or "RESERVED"
    if trimmed.is_empty()
        || trimmed == "0"
        || trimmed == "0.0"
        || upper_value == "OK"
        || upper_value == "RESERVED"
    {
        false
    } else {
        true
    }
}

// Helper function to determine fault severity from DTC fault name
pub fn get_dtc_fault_severity(fault_name: &str) -> FaultSeverity {
    // Check DTC_FLAGS_1_FAULTS
    for &(_, name, severity) in DTC_FLAGS_1_FAULTS.iter() {
        if fault_name.contains(name) {
            return severity;
        }
    }
    
    // Check DTC_FLAGS_2_FAULTS
    for &(_, name, severity) in DTC_FLAGS_2_FAULTS.iter() {
        if fault_name.contains(name) {
            return severity;
        }
    }
    
    // Default severity for unknown DTC faults
    FaultSeverity::Error
}

// Helper function to determine fault severity for non-DTC faults
pub fn get_fault_severity(message_name: &str, signal_name: &str) -> FaultSeverity {
    match message_name {
        "BMS_DTC" => get_dtc_fault_severity(signal_name),
        "MotorController_1" | "MotorController_2" => {
            // Motor controller faults are typically critical
            FaultSeverity::Critical
        }
        "BPS_System" => {
            // BPS faults are safety-critical
            match signal_name {
                "BPS_BMS_CAN_Warning" => FaultSeverity::Warning,
                _ => FaultSeverity::Critical,
            }
        }
        "MPPT" => {
            // MPPT faults are typically errors, not critical
            FaultSeverity::Error
        }
        _ => FaultSeverity::Error, // Default for unknown message types
    }
}
