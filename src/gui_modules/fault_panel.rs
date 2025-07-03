use crate::gui_modules::{get_fault_container_style, Fault, FaultSeverity, Message};
use iced::widget::container::StyleSheet;
use iced::widget::{column, container, row, text, Space};
use iced::{Alignment, Color, Element, Length};
use std::collections::HashMap;

const FAULTS_PER_PAGE: usize = 3;

pub fn fault_display(
    active_faults: &HashMap<String, Fault>,
    current_page: usize,
) -> Element<'static, Message> {
    let fault_count = active_faults.len();

    // Create pagination info
    let total_pages = if fault_count == 0 {
        0
    } else {
        (fault_count + FAULTS_PER_PAGE - 1) / FAULTS_PER_PAGE // Ceiling division
    };

    let page_info = if total_pages > 1 {
        format!(
            "ACTIVE FAULTS: {} (Page {}/{})",
            fault_count,
            current_page + 1,
            total_pages
        )
    } else {
        format!("ACTIVE FAULTS: {}", fault_count)
    };

    // Header showing fault count and page info - will be updated after determining most severe
    let header_text = text(page_info)
        .size(16)
        .horizontal_alignment(iced::alignment::Horizontal::Center);

    if fault_count == 0 {
        // No faults - show simple message
        let header = container(header_text)
            .width(Length::Fill)
            .padding(5)
            .style(iced::theme::Container::Box);

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

    // Sort faults by severity first, then by timestamp (most recent first)
    let mut faults_vec: Vec<_> = active_faults.values().collect();
    faults_vec.sort_by(|a, b| {
        // First sort by severity (Critical -> Error -> Warning)
        match a.severity.cmp(&b.severity) {
            std::cmp::Ordering::Equal => {
                // If same severity, sort by timestamp (most recent first)
                b.timestamp.cmp(&a.timestamp)
            }
            other => other,
        }
    });

    // Determine the most severe fault level for header styling
    let most_severe = faults_vec
        .first()
        .map(|f| &f.severity)
        .unwrap_or(&FaultSeverity::Error);

    // Calculate the range of faults to display for current page
    let start_index = current_page * FAULTS_PER_PAGE;
    let end_index = std::cmp::min(start_index + FAULTS_PER_PAGE, fault_count);

    // Get the faults for the current page
    let current_page_faults = &faults_vec[start_index..end_index];

    // Create list of faults for current page
    let mut fault_list = column![];

    for (idx, fault) in current_page_faults.iter().enumerate() {
        // Create severity-based styling with alternating opacity
        let fault_style = match fault.severity {
            FaultSeverity::Warning => {
                let opacity = if idx % 2 == 0 { 0.15 } else { 0.1 };
                iced::theme::Container::Custom(Box::new(move |theme: &iced::Theme| {
                    let mut appearance = theme.appearance(&iced::theme::Container::Box);
                    appearance.background =
                        Some(Color::from_rgba(238.0, 210.0, 2.0, opacity).into());
                    appearance.border.color = Color::from_rgb(238.0, 210.0, 2.0);
                    appearance.border.width = 1.0;
                    appearance
                }))
            }
            FaultSeverity::Error => {
                let opacity = if idx % 2 == 0 { 0.1 } else { 0.05 };
                iced::theme::Container::Custom(Box::new(move |theme: &iced::Theme| {
                    let mut appearance = theme.appearance(&iced::theme::Container::Box);
                    appearance.background = Some(Color::from_rgba(0.8, 0.0, 0.0, opacity).into());
                    appearance.border.color = Color::from_rgb(0.8, 0.0, 0.0);
                    appearance.border.width = 1.0;
                    appearance
                }))
            }
            FaultSeverity::Critical => {
                let opacity = if idx % 2 == 0 { 0.2 } else { 0.15 };
                iced::theme::Container::Custom(Box::new(move |theme: &iced::Theme| {
                    let mut appearance = theme.appearance(&iced::theme::Container::Box);
                    appearance.background = Some(Color::from_rgba(1.0, 0.0, 0.0, opacity).into());
                    appearance.border.color = Color::from_rgb(1.0, 0.0, 0.0);
                    appearance.border.width = 2.0; // Thicker border for critical
                    appearance
                }))
            }
        };

        let fault_row = container(
            row![
                // Timestamp
                container(text(fault.timestamp.format("%H:%M:%S").to_string()).size(12))
                    .width(Length::FillPortion(1)),
                // Message source
                container(text(&fault.message_name).size(12)).width(Length::FillPortion(1)),
                // Fault name/signal with severity indicator
                container(
                    text(format!(
                        "{} {}",
                        match fault.severity {
                            FaultSeverity::Warning => "⚠",
                            FaultSeverity::Error => "⚠",
                            FaultSeverity::Critical => "🚨",
                        },
                        &fault.name
                    ))
                    .size(12)
                )
                .width(Length::FillPortion(2)),
                // Value
                container(text(&fault.value).size(12)).width(Length::FillPortion(1)),
            ]
            .spacing(5)
            .align_items(Alignment::Center)
            .padding(3),
        )
        .width(Length::Fill)
        .style(fault_style);

        fault_list = fault_list.push(fault_row);
    }

    // Add empty rows to maintain consistent height (always show space for 5 rows)
    let empty_rows_needed = FAULTS_PER_PAGE - current_page_faults.len();
    for _i in 0..empty_rows_needed {
        let empty_row = container(
            row![
                Space::with_width(Length::FillPortion(1)),
                Space::with_width(Length::FillPortion(1)),
                Space::with_width(Length::FillPortion(2)),
                Space::with_width(Length::FillPortion(1)),
            ]
            .spacing(5)
            .padding(3),
        )
        .width(Length::Fill)
        .height(Length::Fixed(25.0)); // Match the height of fault rows

        fault_list = fault_list.push(empty_row);
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

    // Create the header with severity-based styling
    let header = container(header_text)
        .width(Length::Fill)
        .padding(5)
        .style(get_fault_container_style(most_severe));

    // Combine everything - no scrollable needed since we limit to 5 faults
    column![
        header,
        list_header,
        container(fault_list).height(Length::Fixed(125.0)) // Fixed height for exactly 5 rows
    ]
    .spacing(0)
    .into()
}
