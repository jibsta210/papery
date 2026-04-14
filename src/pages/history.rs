use crate::app::Message;
use crate::fl;
use crate::wallpaper::WallpaperInfo;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;
use cosmic::Element;

pub fn view<'a>(
    history: &'a [WallpaperInfo],
    current_index: usize,
    favorites: &'a [String],
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let mut content = widget::Column::new()
        .push(widget::text::title3(fl!("history-title")))
        .push(widget::text::body(fl!("history-description")))
        .push(widget::Space::new().height(spacing.space_m))
        .spacing(spacing.space_xs)
        .width(Length::Fill)
        .padding(spacing.space_m);

    if history.is_empty() {
        content = content.push(
            widget::container(widget::text::body(fl!("history-no-wallpapers")))
                .width(Length::Fill)
                .center(Length::Fill),
        );
    } else {
        for (i, wp) in history.iter().enumerate().rev() {
            let is_current = i == current_index;
            let path_str = wp
                .local_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let is_favorite = favorites.contains(&path_str);

            let source_label = wp.source.display_name();

            let mut row = widget::Row::new()
                .push(
                    widget::Column::new()
                        .push(widget::text::body(wp.title.clone()))
                        .push(widget::text::caption(format!(
                            "{source_label}{}",
                            if is_current { " (current)" } else { "" }
                        )))
                        .width(Length::Fill),
                )
                .align_y(Alignment::Center)
                .spacing(spacing.space_s);

            if !is_current {
                row = row.push(
                    widget::button::icon(widget::icon::from_name(
                        "preferences-desktop-wallpaper-symbolic",
                    ))
                    .on_press(Message::SetWallpaperFromHistory(i))
                    .tooltip(fl!("history-set-as-wallpaper")),
                );
            }

            let fav_icon = if is_favorite {
                "starred-symbolic"
            } else {
                "non-starred-symbolic"
            };
            row = row.push(
                widget::button::icon(widget::icon::from_name(fav_icon))
                    .on_press(Message::ToggleFavorite(i))
                    .tooltip(if is_favorite {
                        fl!("history-remove-from-favorites")
                    } else {
                        fl!("history-add-to-favorites")
                    }),
            );

            content = content.push(
                widget::container(row)
                    .padding(spacing.space_xs)
                    .width(Length::Fill),
            );

            if i > 0 {
                content = content.push(widget::divider::horizontal::default());
            }
        }
    }

    Element::from(widget::scrollable(content))
}
