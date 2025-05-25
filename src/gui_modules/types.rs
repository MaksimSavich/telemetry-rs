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
pub fn get_gui_value_mappings() -> HashMap<(&'static str, &'static str), GuiValueType> {
    let mut mappings = HashMap::new();

    // Motor data - using MotorController_1 for speed and MotorController_2 for direction
    mappings.insert(
        ("MotorController_1", "Actual_Speed_RPM"),
        GuiValueType::Speed,
    );
    mappings.insert(
        ("MotorController_2", "Status_Of_Command"),
        GuiValueType::Direction,
    );

    // BMS data
    mappings.insert(("BMS_Limits", "Pack_DCL"), GuiValueType::BmsPackDcl);
    mappings.insert(("BMS_Limits", "Pack_DCL_KW"), GuiValueType::BmsPackDclKw);
    mappings.insert(("BMS_Limits", "Pack_CCL"), GuiValueType::BmsPackCcl);
    mappings.insert(("BMS_Limits", "Pack_CCL_KW"), GuiValueType::BmsPackCclKw);
    mappings.insert(("BMS_State", "Pack_DOD"), GuiValueType::BmsPackDod);
    mappings.insert(("BMS_State", "Pack_Health"), GuiValueType::BmsPackHealth);
    mappings.insert(("BMS_State", "Adaptive_SOC"), GuiValueType::BmsAdaptiveSoc);
    mappings.insert(("BMS_State", "Pack_SOC"), GuiValueType::BmsPackSoc);
    mappings.insert(
        ("BMS_Capacity", "Adaptive_Amphours"),
        GuiValueType::BmsAdaptiveAmphours,
    );
    mappings.insert(
        ("BMS_Capacity", "Pack_Amphours"),
        GuiValueType::BmsPackAmphours,
    );

    // Battery/BPS data
    mappings.insert(
        ("BMS_Power", "Pack_Summed_Voltage"),
        GuiValueType::BatteryVoltage,
    );
    mappings.insert(("BMS_Power", "Pack_Current"), GuiValueType::BatteryCurrent);
    mappings.insert(("BMS_State", "Adaptive_SOC"), GuiValueType::BatteryCharge); // Using Adaptive SOC as charge level
    mappings.insert(
        ("BPS_System", "Supp_Temperature_C"),
        GuiValueType::BatteryTemp,
    );
    mappings.insert(("BPS_System", "BPS_ON_Time"), GuiValueType::BpsOnTime);
    mappings.insert(("BPS_System", "BPS_State"), GuiValueType::BpsState);

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
            "BPS_CAN",
            "BPS_Precharge",
            "BPS_Main_Pack_Voltage",
            "BPS_Current",
            "BPS_DCDC_Voltage",
            "BPS_Supp_Temperature",
            "BPS_Supp_Voltage",
            "BPS_Val1",
            "BPS_Val2",
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
