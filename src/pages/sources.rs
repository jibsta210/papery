use crate::app::Message;
use crate::config::PaperyConfig;
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

fn category_chip<'a>(label: &'a str, enabled: bool, on_press: Message) -> Element<'a, Message> {
    let style = if enabled {
        cosmic::theme::Button::Suggested
    } else {
        cosmic::theme::Button::Standard
    };
    widget::button::text(label.to_string())
        .on_press(on_press)
        .class(style)
        .into()
}

pub fn view(config: &PaperyConfig) -> Element<'_, Message> {
    let spacing = cosmic::theme::spacing();

    let content = widget::Column::new()
        .push(widget::text::title3(fl!("sources-title")))
        .push(widget::text::body(fl!("sources-description")))
        .push(widget::Space::new().height(spacing.space_m))
        .push(source_toggle(
            fl!("source-bing"),
            fl!("source-bing-description"),
            config.source_bing,
            Message::ToggleSourceBing,
        ))
        .push(source_toggle(
            fl!("source-nasa"),
            fl!("source-nasa-description"),
            config.source_nasa,
            Message::ToggleSourceNasa,
        ))
        .push(source_toggle(
            fl!("source-wallhaven"),
            fl!("source-wallhaven-description"),
            config.source_wallhaven,
            Message::ToggleSourceWallhaven,
        ))
        .push_maybe(if config.source_wallhaven {
            Some(
                widget::Row::new()
                    .push(widget::Space::new().width(spacing.space_l))
                    .push(category_chip(
                        "General",
                        config.wallhaven_general,
                        Message::ToggleWallhavenGeneral,
                    ))
                    .push(category_chip(
                        "Anime",
                        config.wallhaven_anime,
                        Message::ToggleWallhavenAnime,
                    ))
                    .push(category_chip(
                        "People",
                        config.wallhaven_people,
                        Message::ToggleWallhavenPeople,
                    ))
                    .spacing(spacing.space_xs)
                    .align_y(Alignment::Center),
            )
        } else {
            None
        })
        .push(source_toggle(
            fl!("source-earthview"),
            fl!("source-earthview-description"),
            config.source_earthview,
            Message::ToggleSourceEarthView,
        ))
        .push(source_toggle(
            fl!("source-local"),
            fl!("source-local-description"),
            config.source_local,
            Message::ToggleSourceLocal,
        ))
        .push(source_toggle(
            "Unsplash".to_string(),
            "High-quality photography (no API key needed)".to_string(),
            config.source_unsplash,
            Message::ToggleSourceUnsplash,
        ))
        .push(source_toggle(
            "Pexels".to_string(),
            "Curated stock photography (API key required, free)".to_string(),
            config.source_pexels,
            Message::ToggleSourcePexels,
        ))
        .spacing(spacing.space_xs)
        .width(Length::Fill)
        .padding(spacing.space_m);

    Element::from(widget::scrollable(content))
}

fn source_toggle<'a>(
    name: String,
    description: String,
    enabled: bool,
    on_toggle: Message,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    widget::settings::item_row(vec![
        widget::Column::new()
            .push(widget::text::body(name))
            .push(widget::text::caption(description))
            .spacing(spacing.space_xxxs)
            .width(Length::Fill)
            .into(),
        widget::toggler(enabled)
            .on_toggle(move |_| on_toggle.clone())
            .into(),
    ])
    .align_y(Alignment::Center)
    .into()
}
