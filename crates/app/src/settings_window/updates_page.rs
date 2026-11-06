use gpui::{App, FontWeight, IntoElement, div, prelude::*};
use qd_update::AppVersion;

use super::setting_row::SettingRow;
use crate::app_update::{AppUpdater, UpdateStatus};
use crate::button::Button;
use crate::markdown_view::MarkdownView;
use crate::theme;

/// The Updates page: the running version and where the update stands, a
/// check on demand and, once a newer version is ready, its notes and the
/// restart that installs it.
pub fn render_updates_page(cx: &App) -> impl IntoElement {
    let updater = AppUpdater::global(cx);
    let status = updater.read(cx).status().clone();
    let is_checking = matches!(
        status,
        UpdateStatus::Checking | UpdateStatus::Downloading(_)
    );
    let check_button = Button::new(
        "check-for-updates",
        if is_checking {
            "Checking…"
        } else {
            "Check now"
        },
    )
    .disabled(!status.can_check())
    .on_click({
        let updater = updater.clone();
        move |_, _, cx| updater.update(cx, AppUpdater::check_now)
    });

    div()
        .flex()
        .flex_col()
        .child(SettingRow::new(
            format!("Quarterdeck {}", AppVersion::current()),
            status.describe(),
            check_button,
        ))
        .children(match status {
            UpdateStatus::Ready(ready) => Some(
                div()
                    .flex()
                    .flex_col()
                    .child(SettingRow::new(
                        format!("Version {} is ready", ready.release.version),
                        "Restarting installs it now; otherwise it installs when you quit.",
                        Button::new("restart-to-update", "Restart to update")
                            .primary()
                            .on_click(move |_, _, cx| {
                                updater.update(cx, AppUpdater::restart_to_update);
                            }),
                    ))
                    .child(
                        div()
                            .pt_4()
                            .pb_2()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme::subtle())
                            .child("WHAT'S NEW"),
                    )
                    .child(MarkdownView::new("release-notes", ready.notes)),
            ),
            _ => None,
        })
}
