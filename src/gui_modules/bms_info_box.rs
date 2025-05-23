use crate::gui_modules::Message;
use iced::widget::{column, container, text};
use iced::{Alignment, Element, Length};

#[derive(Clone)]
pub struct BmsData {
    pub pack_dcl: f64,
    pub pack_dcl_kw: f64,
    pub pack_ccl: f64,
    pub pack_ccl_kw: f64,
    pub pack_dod: f64,
    pub pack_health: f64,
    pub adaptive_soc: f64,
    pub pack_soc: f64,
    pub adaptive_amphours: f64,
    pub pack_amphours: f64,
}

impl Default for BmsData {
    fn default() -> Self {
        Self {
            pack_dcl: 0.0,
            pack_dcl_kw: 0.0,
            pack_ccl: 0.0,
            pack_ccl_kw: 0.0,
            pack_dod: 0.0,
            pack_health: 0.0,
            adaptive_soc: 0.0,
            pack_soc: 0.0,
            adaptive_amphours: 0.0,
            pack_amphours: 0.0,
        }
    }
}

pub fn bms_info_box(data: &BmsData) -> Element<'static, Message> {
    container(
        column![
            text("BMS Info").size(18),
            text(format!(
                "DCL: {:.0} A / {:.1} kW",
                data.pack_dcl, data.pack_dcl_kw
            ))
            .size(14),
            text(format!(
                "CCL: {:.0} A / {:.1} kW",
                data.pack_ccl, data.pack_ccl_kw
            ))
            .size(14),
            text(format!("DOD: {:.1}%", data.pack_dod)).size(14),
            text(format!("Health: {:.0}%", data.pack_health)).size(14),
            text(format!(
                "SOC: {:.1}% (Adaptive: {:.1}%)",
                data.pack_soc, data.adaptive_soc
            ))
            .size(14),
            text(format!(
                "Ah: {:.1} (Adaptive: {:.1})",
                data.pack_amphours, data.adaptive_amphours
            ))
            .size(14),
        ]
        .spacing(4)
        .align_items(Alignment::Start),
    )
    .padding(10)
    .width(Length::FillPortion(1))
    .style(iced::theme::Container::Box)
    .into()
}
