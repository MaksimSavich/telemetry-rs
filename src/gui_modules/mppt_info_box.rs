use crate::gui_modules::Message;
use iced::widget::{column, container, row, text};
use iced::{Alignment, Element, Length};

#[derive(Clone)]
pub struct MpptData {
    pub mppt1_input_voltage: f64,
    pub mppt1_input_current: f64,
    pub mppt1_output_voltage: f64,
    pub mppt1_output_current: f64,
    pub mppt2_input_voltage: f64,
    pub mppt2_input_current: f64,
    pub mppt2_output_voltage: f64,
    pub mppt2_output_current: f64,
}

impl Default for MpptData {
    fn default() -> Self {
        Self {
            mppt1_input_voltage: 0.0,
            mppt1_input_current: 0.0,
            mppt1_output_voltage: 0.0,
            mppt1_output_current: 0.0,
            mppt2_input_voltage: 0.0,
            mppt2_input_current: 0.0,
            mppt2_output_voltage: 0.0,
            mppt2_output_current: 0.0,
        }
    }
}

pub fn mppt_info_box(data: &MpptData) -> Element<'static, Message> {
    container(
        column![
            text("MPPT & BPS Info").size(18),
            row![
                // MPPT 1 Column
                column![
                    text("MPPT Back").size(16),
                    text(format!(
                        "In: {:.1}V / {:.1}A",
                        data.mppt1_input_voltage, data.mppt1_input_current
                    ))
                    .size(14),
                    text(format!(
                        "Out: {:.1}V / {:.1}A",
                        data.mppt1_output_voltage, data.mppt1_output_current
                    ))
                    .size(14),
                ]
                .spacing(4)
                .align_items(Alignment::Start)
                .width(Length::FillPortion(1)),
                // MPPT 2 Column
                column![
                    text("MPPT Front").size(16),
                    text(format!(
                        "In: {:.1}V / {:.1}A",
                        data.mppt2_input_voltage, data.mppt2_input_current
                    ))
                    .size(14),
                    text(format!(
                        "Out: {:.1}V / {:.1}A",
                        data.mppt2_output_voltage, data.mppt2_output_current
                    ))
                    .size(14),
                ]
                .spacing(4)
                .align_items(Alignment::Start)
                .width(Length::FillPortion(1)),
                column![
                    text("BPS Info").size(20),
                    text(format!("Time On: {:.1} Seconds", data.ontime)),
                    text(format!("BPS State: {}", data.state)),
                ]
                .spacing(5)
                .align_items(Alignment::Start),
            ]
            .spacing(10)
        ]
        .spacing(8)
        .align_items(Alignment::Start),
    )
    .padding(10)
    .width(Length::FillPortion(1))
    .style(iced::theme::Container::Box)
    .into()
}
