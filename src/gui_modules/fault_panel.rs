use crate::gui_modules::{create_error_container_style, Fault, Message};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element};
use std::collections::HashMap;

pub fn fault_section(active_faults: &HashMap<String, Fault>) -> Element<'static, Message> {
    // Fault Indicator
    let fault_indicator = container(text("FAULT"))
        .style(if !active_faults.is_empty() {
            create_error_container_style()
        } else {
            iced::theme::Container::Box
        })
        .padding(10);

    // Fault List
    let fault_list = column(
        active_faults
            .values()
            .map(|fault| {
                let name = fault.name.clone();
                let value = fault.value.clone();
                text(format!("{}: {} (Active)", name, value)).into()
            })
            .collect::<Vec<_>>(),
    )
    .spacing(5);

    // Clear Faults Button
    let clear_faults_button = button("Clear Faults").on_press(Message::ClearFaults);

    // Combine into fault section
    column![
        row![fault_indicator, fault_list]
            .spacing(10)
            .align_items(Alignment::Center),
        clear_faults_button
    ]
    .spacing(10)
    .into()
}
