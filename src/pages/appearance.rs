use crate::app::Message;
use crate::config::PaperyConfig;
use crate::fl;
use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

pub fn view(config: &PaperyConfig) -> Element<'_, Message> {
    let spacing = cosmic::theme::spacing();

    let content = widget::Column::new()
        .push(widget::text::title3(fl!("appearance-title")))
        .push(widget::text::body(fl!("appearance-description")))
        .push(widget::Space::new().height(spacing.space_m))
        // Scaling mode
        .push(widget::text::heading(fl!("appearance-scaling")))
        .push(
            widget::Row::new()
                .push(scaling_button(
                    "zoom",
                    fl!("appearance-scaling-zoom"),
                    config,
                ))
                .push(scaling_button(
                    "fit",
                    fl!("appearance-scaling-fit"),
                    config,
                ))
                .push(scaling_button(
                    "stretch",
                    fl!("appearance-scaling-stretch"),
                    config,
                ))
                .spacing(spacing.space_xs),
        )
        .push(widget::Space::new().height(spacing.space_m))
        // Theme filter
        .push(widget::text::heading(fl!("appearance-theme-filter")))
        .push(
            widget::Row::new()
                .push(theme_button("any", fl!("appearance-theme-any"), config))
                .push(theme_button("light", fl!("appearance-theme-light"), config))
                .push(theme_button("dark", fl!("appearance-theme-dark"), config))
                .spacing(spacing.space_xs),
        )
        .push(widget::Space::new().height(spacing.space_m))
        // Brightness threshold
        .push(widget::settings::item_row(vec![
            widget::text::body(fl!("appearance-brightness-threshold"))
                .width(Length::Fill)
                .into(),
            widget::slider(0.0..=1.0, config.brightness_threshold, |val| {
                Message::SetBrightnessThreshold(val)
            })
            .step(0.05)
            .width(Length::Fixed(300.0))
            .into(),
        ]))
        .spacing(spacing.space_xs)
        .width(Length::Fill)
        .padding(spacing.space_m);

    Element::from(widget::scrollable(content))
}

fn scaling_button<'a>(
    mode: &'static str,
    label: String,
    config: &PaperyConfig,
) -> Element<'a, Message> {
    let is_active = config.scaling_mode == mode;
    let style = if is_active {
        cosmic::theme::Button::Suggested
    } else {
        cosmic::theme::Button::Standard
    };
    widget::button::text(label)
        .on_press(Message::SetScalingMode(mode.to_string()))
        .class(style)
        .into()
}

fn theme_button<'a>(
    filter: &'static str,
    label: String,
    config: &PaperyConfig,
) -> Element<'a, Message> {
    let is_active = config.theme_filter == filter;
    let style = if is_active {
        cosmic::theme::Button::Suggested
    } else {
        cosmic::theme::Button::Standard
    };
    widget::button::text(label)
        .on_press(Message::SetThemeFilter(filter.to_string()))
        .class(style)
        .into()
}
