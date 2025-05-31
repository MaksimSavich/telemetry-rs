use chrono::{DateTime, Utc};
use iced::{widget::container::StyleSheet, Color, Theme};
use socketcan::CanFrame;
use std::collections::HashMap;

// Re-export common types and messages for all components

#[derive(Clone, Debug)]
pub struct Fault {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub is_active: bool,
    pub value: String,
    pub message_name: String, // Added to track which CAN message this came from
}

// Message enum shared between all components
#[derive(Debug, Clone)]
pub enum Message {
    CanFrameReceived(String, CanFrame),
    ToggleFullscreen,
    Tick, // For updating time display
}

// Container styling helper
pub fn create_error_container_style() -> iced::theme::Container {
    iced::theme::Container::Custom(Box::new(move |theme: &Theme| {
        let mut appearance = theme.appearance(&iced::theme::Container::Box);
        appearance.background = Some(Color::from_rgb(1.0, 0.0, 0.0).into());
        appearance.text_color = Some(Color::WHITE);
        appearance
    }))
}

// DTC fault definitions for BMS (existing - keep as is)
pub const DTC_FLAGS_1_FAULTS: &[(u16, &str)] = &[
    (0x0001, "Discharge Limit Enforcement"),
    (0x0002, "Charger Safety Relay"),
    (0x0004, "Internal Hardware"),
    (0x0008, "Internal Heatsink Thermistor"),
    (0x0010, "Internal Software"),
    (0x0020, "Highest Cell Voltage Too High"),
    (0x0040, "Lowest Cell Voltage Too Low"),
    (0x0080, "Pack Too Hot"),
];

pub const DTC_FLAGS_2_FAULTS: &[(u16, &str)] = &[
    (0x0001, "Internal Communication"),
    (0x0002, "Cell Balancing Stuck Off"),
    (0x0004, "Weak Cell"),
    (0x0008, "Low Cell Voltage"),
    (0x0010, "Open Wiring"),
    (0x0020, "Current Sensor"),
    (0x0040, "Highest Cell Voltage Over 5V"),
    (0x0080, "Cell ASIC"),
    (0x0100, "Weak Pack"),
    (0x0200, "Fan Monitor"),
    (0x0400, "Thermistor"),
    (0x0800, "External Communication"),
    (0x1000, "Redundant Power Supply"),
    (0x2000, "High Voltage Isolation"),
    (0x4000, "Input Power Supply"),
    (0x8000, "Charge Limit Enforcement"),
];

// Configuration for GUI value updates using (message_name, signal_name) as key
// Returns a Vec of GuiValueType since one signal might update multiple GUI values
// Returns a Vec of GuiValueType since one signal might update multiple GUI values
pub fn get_gui_value_mappings() -> HashMap<(&'static str, &'static str), Vec<GuiValueType>> {
    let mut mappings = HashMap::new();

    // Motor data - using MotorController_1 for speed and MotorController_2 for direction
    mappings.insert(
        ("MotorController_1", "Actual_Speed_RPM"),
        vec![GuiValueType::Speed],
    );
    mappings.insert(
        ("MotorController_2", "Status_Of_Command"),
        vec![GuiValueType::Direction],
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
        ("BMS_Power", "Pack_Summed_Voltage"),
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
    mappings.insert(("BPS_System", "BPS_ON_Time"), vec![GuiValueType::BpsOnTime]);
    mappings.insert(("BPS_System", "BPS_State"), vec![GuiValueType::BpsState]);

    mappings
}

// Enum for different GUI value types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GuiValueType {
    Speed,
    Direction,
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
