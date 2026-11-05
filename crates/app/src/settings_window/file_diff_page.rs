use gpui::{IntoElement, div, prelude::*};
use qc_cache::{DiffLayout, DiffViewSettings, LineWrapping};
use qc_diff::{ChangeHighlight, WhitespaceMode};

use super::setting_row::SettingRow;
use crate::app_settings::AppSettings;
use crate::segmented_control::SegmentedControl;

/// The File Diff page: layout, change highlighting, whitespace and long
/// lines, each applied to every open diff as soon as it is chosen.
pub fn render_file_diff_page(settings: DiffViewSettings) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(SettingRow::new(
            "Layout",
            "Show removed and added lines in one column, or old and new side by side.",
            diff_choice(
                "layout",
                settings,
                settings.layout,
                &[
                    ("Unified", DiffLayout::Unified),
                    ("Split", DiffLayout::Split),
                ],
                |settings, layout| DiffViewSettings { layout, ..settings },
            ),
        ))
        .child(SettingRow::new(
            "Change highlighting",
            "How finely the changes inside a changed line are marked.",
            diff_choice(
                "highlight",
                settings,
                settings.highlight,
                &[
                    ("Lines", ChangeHighlight::Lines),
                    ("Words", ChangeHighlight::Words),
                    ("Characters", ChangeHighlight::Characters),
                ],
                |settings, highlight| DiffViewSettings {
                    highlight,
                    ..settings
                },
            ),
        ))
        .child(SettingRow::new(
            "Whitespace changes",
            "Hiding treats lines that only changed in indentation or spacing as unchanged.",
            diff_choice(
                "whitespace",
                settings,
                settings.whitespace,
                &[
                    ("Show", WhitespaceMode::Show),
                    ("Hide", WhitespaceMode::Ignore),
                ],
                |settings, whitespace| DiffViewSettings {
                    whitespace,
                    ..settings
                },
            ),
        ))
        .child(SettingRow::new(
            "Long lines",
            "Wrap long lines, or keep each on one line and scroll a file sideways.",
            diff_choice(
                "wrapping",
                settings,
                settings.wrapping,
                &[
                    ("Wrap", LineWrapping::Wrap),
                    ("Scroll", LineWrapping::Scroll),
                ],
                |settings, wrapping| DiffViewSettings {
                    wrapping,
                    ..settings
                },
            ),
        ))
}

/// Exclusive options for one diff setting: each `(label, value)` shows
/// selected when it is `current`, and choosing it saves `apply(settings, value)`.
fn diff_choice<T: Copy + PartialEq + 'static>(
    id: &'static str,
    settings: DiffViewSettings,
    current: T,
    options: &[(&'static str, T)],
    apply: fn(DiffViewSettings, T) -> DiffViewSettings,
) -> SegmentedControl {
    options
        .iter()
        .fold(SegmentedControl::new(id), |control, &(label, value)| {
            let chosen = apply(settings, value);
            control.option(label, value == current, move |_, cx| {
                AppSettings::set_diff_view(cx, chosen);
            })
        })
}
