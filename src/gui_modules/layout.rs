use crate::gui_modules::Message;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

pub fn main_layout<'a>(
    is_fullscreen: bool,
    can_status: Element<'a, Message>,
    radio_status: Element<'a, Message>,
    mppt_bps_info: Element<'a, Message>,
    speed_direction: Element<'a, Message>,
    battery_info: Element<'a, Message>,
    fault_display: Element<'a, Message>,
    time_display: Element<'a, Message>,
    warning_indicator: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    // Top row: CAN status (left), spacer, radio status, fullscreen button (right)
    let top_row = container(
        row![
            can_status,
            Space::with_width(Length::Fill),
            radio_status,
            container(
                button(
                    text(if is_fullscreen {
                        "Exit Fullscreen"
                    } else {
                        "Fullscreen"
                    })
                    .size(14)
                )
                .on_press(Message::ToggleFullscreen),
            )
            .width(Length::Shrink)
        ]
        .spacing(10)
        .align_items(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(40.0))
    .padding([5, 10]);

    // Main info row: MPPT/BPS info (left), speed/direction (center), battery info (right)
    // Fixed height to prevent shrinking
    let main_info_row = container(
        row![mppt_bps_info, speed_direction, battery_info,]
            .spacing(5)
            .align_items(Alignment::Center),
    )
    .width(Length::Fill)
    .height(Length::Fixed(220.0)) // Fixed height for consistency
    .padding([5, 10]);

    // Fault display row with fixed container height
    // The fault panel inside can scroll, but the container stays the same size
    let fault_row = container(fault_display)
        .width(Length::Fill)
        .height(Length::Fixed(135.0)) // Fixed height container
        .padding([0, 10]);

    // Bottom row with time and optional warning
    let bottom_row = if let Some(warning) = warning_indicator {
        container(
            row![warning, time_display,]
                .spacing(5)
                .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fixed(70.0))
    } else {
        container(time_display)
            .width(Length::Fill)
            .height(Length::Fixed(50.0))
    };

    // Combine all sections with fixed layout
    // Total height: 40 + 220 + 80 + 180 + 30 = 550px (leaving 50px for spacing on 600px display)
    column![
        top_row,
        main_info_row,
        fault_row,
        Space::with_height(Length::Fill), // This will absorb any extra space
        bottom_row,
    ]
    .spacing(10)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
