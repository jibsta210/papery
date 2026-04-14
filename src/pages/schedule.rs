use crate::app::{Message, Papery};
use crate::fl;
use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

pub fn view(app: &Papery) -> Element<'_, Message> {
    let config = &app.config;
    let spacing = cosmic::theme::spacing();

    let remaining_mins = app.seconds_until_next / 60;
    let remaining_secs = app.seconds_until_next % 60;
    let countdown_text = if config.paused {
        fl!("schedule-paused")
    } else {
        format!("{remaining_mins}:{remaining_secs:02}")
    };

    let content = widget::Column::new()
        .push(widget::text::title3(fl!("schedule-title")))
        .push(widget::text::body(fl!("schedule-description")))
        .push(widget::Space::new().height(spacing.space_m))
        // H:M:S interval input
        .push(widget::text::heading(fl!("schedule-interval")))
        .push(widget::Space::new().height(spacing.space_xxs))
        .push(
            widget::Row::new()
                .push(
                    widget::text_input::text_input("0", &app.interval_hours)
                        .on_input(Message::SetIntervalHours)
                        .width(Length::Fixed(60.0)),
                )
                .push(widget::text::body("h"))
                .push(widget::Space::new().width(spacing.space_s))
                .push(
                    widget::text_input::text_input("0", &app.interval_minutes)
                        .on_input(Message::SetIntervalMinutes)
                        .width(Length::Fixed(60.0)),
                )
                .push(widget::text::body("m"))
                .push(widget::Space::new().width(spacing.space_s))
                .push(
                    widget::text_input::text_input("0", &app.interval_seconds)
                        .on_input(Message::SetIntervalSeconds)
                        .width(Length::Fixed(60.0)),
                )
                .push(widget::text::body("s"))
                .spacing(spacing.space_xxs)
                .align_y(cosmic::iced::Alignment::Center),
        )
        .push(widget::Space::new().height(spacing.space_m))
        // Pause/resume
        .push(widget::settings::item_row(vec![
            widget::text::body(if config.paused {
                fl!("schedule-resume")
            } else {
                fl!("schedule-pause")
            })
            .width(Length::Fill)
            .into(),
            widget::toggler(!config.paused)
                .on_toggle(|_| Message::TogglePause)
                .into(),
        ]))
        .push(widget::Space::new().height(spacing.space_m))
        // Countdown display
        .push(widget::settings::item_row(vec![
            widget::text::body(fl!("schedule-next-in"))
                .width(Length::Fill)
                .into(),
            widget::text::title4(countdown_text).into(),
        ]))
        .spacing(spacing.space_xs)
        .width(Length::Fill)
        .padding(spacing.space_m);

    Element::from(widget::scrollable(content))
}
