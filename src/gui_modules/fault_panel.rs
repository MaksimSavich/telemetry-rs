use crate::gui_modules::{create_error_container_style, Fault, Message};
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length};
use std::collections::HashMap;

pub fn fault_section(
    active_faults: &HashMap<String, Fault>,
    expanded: bool,
    current_fault_index: usize,
) -> Element<'static, Message> {
    let fault_count = active_faults.len();
    let has_faults = fault_count > 0;

    // Create the header section with fault count and expand/collapse button
    let header = {
        let fault_indicator = container(text(format!("Faults: {}", fault_count)).size(16))
            .style(if has_faults {
                create_error_container_style()
            } else {
                iced::theme::Container::Box
            })
            .padding(10)
            .width(Length::FillPortion(1));

        let expand_button = button(text(if expanded { "Collapse" } else { "Expand" }).size(14))
            .on_press(Message::ToggleFaultPanelExpanded)
            .width(Length::Shrink);

        row![fault_indicator, expand_button]
            .spacing(10)
            .align_items(Alignment::Center)
    };

    // The main content depends on expanded state
    let content = if !has_faults {
        // No faults to display
        Element::from(
            container(text("No active faults").size(14))
                .width(Length::Fill)
                .center_x()
                .padding(10),
        )
    } else if expanded {
        // Expanded view - show all faults in a scrollable list
        let faults_list = active_faults
            .values()
            .map(|fault| {
                row![
                    text(&fault.name).width(Length::FillPortion(3)),
                    text(&fault.value).width(Length::FillPortion(1)),
                    text(format!("{}", fault.timestamp.format("%H:%M:%S")))
                        .width(Length::FillPortion(1))
                ]
                .spacing(10)
                .padding(5)
                .width(Length::Fill)
                .into()
            })
            .collect::<Vec<_>>();

        // Add header row for the fault list
        let header_row = row![
            text("Fault Name").width(Length::FillPortion(3)).size(14),
            text("Value").width(Length::FillPortion(1)).size(14),
            text("Time").width(Length::FillPortion(1)).size(14)
        ]
        .spacing(10)
        .padding(5);

        // Combine header and scrollable content
        let scroll_content = column![header_row]
            .push(
                scrollable(column(faults_list).spacing(2).width(Length::Fill))
                    .height(Length::Fixed(150.0)),
            )
            .spacing(5)
            .width(Length::Fill);

        container(scroll_content)
            .width(Length::Fill)
            .padding(10)
            .style(iced::theme::Container::Box)
            .into()
    } else {
        // Collapsed view - show cycling fault at current index
        let current_fault = if has_faults {
            let faults_vec: Vec<_> = active_faults.values().collect();
            if current_fault_index < faults_vec.len() {
                Some(faults_vec[current_fault_index])
            } else {
                None
            }
        } else {
            None
        };

        // Display current fault or a message
        if let Some(fault) = current_fault {
            // Left side (25%) shows count, right side (75%) shows cycling fault
            row![
                container(
                    text(format!("{}/{}", current_fault_index + 1, fault_count))
                        .size(16)
                        .horizontal_alignment(iced::alignment::Horizontal::Center)
                )
                .width(Length::FillPortion(1))
                .center_x()
                .center_y(),
                container(
                    row![
                        text(&fault.name).width(Length::FillPortion(3)),
                        text(&fault.value).width(Length::FillPortion(1)),
                    ]
                    .spacing(10)
                    .align_items(Alignment::Center)
                )
                .width(Length::FillPortion(3))
                .padding(10)
            ]
            .spacing(5)
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .into()
        } else {
            // Fallback (should never happen with has_faults check above)
            container(text("No active faults").size(14))
                .width(Length::Fill)
                .center_x()
                .padding(10)
                .into()
        }
    };

    // Clear faults button
    let clear_button = button("Clear Faults")
        .on_press(Message::ClearFaults)
        .width(Length::Shrink);

    // Combine everything
    column![
        header,
        content,
        container(clear_button).width(Length::Fill).center_x(),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}
