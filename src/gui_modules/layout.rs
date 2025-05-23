use crate::gui_modules::Message;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length};

pub fn main_layout<'a>(
    is_fullscreen: bool,
    can_status: Element<'a, Message>,
    radio_status: Element<'a, Message>,
    bms_info: Element<'a, Message>,
    speed_direction: Element<'a, Message>,
    battery_info: Element<'a, Message>,
    bps_info: Element<'a, Message>,
    fault_display: Element<'a, Message>,
    time_display: Element<'a, Message>,
) -> Element<'a, Message> {
    // Top row: CAN status (left), spacer, radio status, fullscreen button (right)
    let top_row = row![
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
    .align_items(Alignment::Center)
    .padding([5, 10]);

    // Main info row: BMS info (left), speed/direction (center), battery info (right)
    let main_info_row = row![bms_info, speed_direction, battery_info,]
        .spacing(10)
        .align_items(Alignment::Center)
        .height(Length::FillPortion(2));

    // BPS info row
    let bps_row = container(bps_info).width(Length::Fill).padding([0, 10]);

    // Fault display row
    let fault_row = container(fault_display)
        .width(Length::Fill)
        .padding([0, 10]);

    // Bottom row with time
    let bottom_row = container(time_display).width(Length::Fill);

    // Combine all sections
    column![
        top_row,
        main_info_row,
        bps_row,
        fault_row,
        Space::with_height(Length::Fill),
        bottom_row,
    ]
    .spacing(5)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
