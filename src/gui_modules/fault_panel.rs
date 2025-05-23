use crate::gui_modules::{create_error_container_style, Fault, Message};
use iced::widget::container::{Appearance, StyleSheet};
use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length};
use std::collections::HashMap;

pub fn fault_display(active_faults: &HashMap<String, Fault>) -> Element<'static, Message> {
    let fault_count = active_faults.len();

    // Header showing fault count
    let header = container(
        text(format!("ACTIVE FAULTS: {}", fault_count))
            .size(16)
            .horizontal_alignment(iced::alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .padding(5)
    .style(if fault_count > 0 {
        create_error_container_style()
    } else {
        iced::theme::Container::Box
    });

    if fault_count == 0 {
        // No faults - show simple message
        return column![
            header,
            container(
                text("System OK - No Active Faults")
                    .size(14)
                    .horizontal_alignment(iced::alignment::Horizontal::Center)
            )
            .width(Length::Fill)
            .padding(10)
            .center_x()
        ]
        .spacing(2)
        .into();
    }

    // Create scrollable list of faults
    let mut fault_list = column![];
    let mut faults_vec: Vec<_> = active_faults.values().collect();
    faults_vec.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Most recent first

    for (idx, fault) in faults_vec.iter().enumerate() {
        let fault_row = container(
            row![
                // Timestamp
                container(text(fault.timestamp.format("%H:%M:%S").to_string()).size(12))
                    .width(Length::FillPortion(1)),
                // Message source
                container(text(&fault.message_name).size(12)).width(Length::FillPortion(1)),
                // Fault name/signal
                container(text(&fault.name).size(12)).width(Length::FillPortion(2)),
                // Value
                container(text(&fault.value).size(12)).width(Length::FillPortion(1)),
            ]
            .spacing(5)
            .align_items(Alignment::Center)
            .padding(3),
        )
        .width(Length::Fill)
        .style(
            // Alternate row colors for better readability
            if idx % 2 == 0 {
                iced::theme::Container::Custom(Box::new(|theme: &iced::Theme| {
                    let mut appearance = theme.appearance(&iced::theme::Container::Box);
                    appearance.background = Some(Color::from_rgba(0.8, 0.0, 0.0, 0.1).into());
                    appearance.border.color = Color::from_rgb(0.8, 0.0, 0.0);
                    appearance.border.width = 1.0;
                    appearance
                }))
            } else {
                iced::theme::Container::Custom(Box::new(|theme: &iced::Theme| {
                    let mut appearance = theme.appearance(&iced::theme::Container::Box);
                    appearance.background = Some(Color::from_rgba(0.8, 0.0, 0.0, 0.05).into());
                    appearance.border.color = Color::from_rgb(0.8, 0.0, 0.0);
                    appearance.border.width = 1.0;
                    appearance
                }))
            },
        );

        fault_list = fault_list.push(fault_row);
    }

    // Create header row for the fault list
    let list_header = container(
        row![
            text("Time").size(12).width(Length::FillPortion(1)),
            text("Source").size(12).width(Length::FillPortion(1)),
            text("Fault").size(12).width(Length::FillPortion(2)),
            text("Value").size(12).width(Length::FillPortion(1)),
        ]
        .spacing(5)
        .padding(3),
    )
    .width(Length::Fill)
    .style(iced::theme::Container::Custom(Box::new(
        |theme: &iced::Theme| {
            let mut appearance = theme.appearance(&iced::theme::Container::Box);
            appearance.background = Some(Color::from_rgb(0.2, 0.2, 0.2).into());
            appearance.text_color = Some(Color::WHITE);
            appearance
        },
    )));

    // Combine everything with scrollable area
    column![
        header,
        list_header,
        scrollable(fault_list).height(Length::Fixed(120.0)) // Fixed height for fault display
    ]
    .spacing(0)
    .into()
}
