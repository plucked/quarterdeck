use std::rc::Rc;

use gpui::{App, ClickEvent, Div, FontWeight, Rgba, Stateful, Window, div, prelude::*, px};
use qc_github::PullRequestFile;

use super::FileDiffState;
use crate::checkbox::{CheckState, Checkbox};
use crate::theme;

/// Edge length of the file status badge.
const STATUS_BADGE_SIZE: f32 = 15.;

/// A click handler shared by several elements of one file.
pub type FileClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The file's header row, the top of its card: chevron, status, path
/// (old → new for renames), line counts and the viewed checkbox. Clicking
/// the row collapses or expands the file; clicking the checkbox only
/// toggles viewed.
pub fn render_file_header(
    index: usize,
    file: &PullRequestFile,
    state: FileDiffState,
    on_toggle_collapsed: FileClickHandler,
    on_toggle_viewed: FileClickHandler,
) -> Stateful<Div> {
    let chevron = if state.is_collapsed { "▸" } else { "▾" };
    let (directory, name) = split_path(&file.filename);
    let renamed_from = file
        .previous_filename
        .as_ref()
        .filter(|previous| **previous != file.filename);

    div()
        .id(("file-header", index))
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .py_1p5()
        .bg(theme::surface())
        .border_1()
        .rounded_t_md()
        .border_color(theme::border())
        .text_sm()
        .cursor_pointer()
        .on_click(move |event, window, cx| on_toggle_collapsed(event, window, cx))
        .child(
            div()
                .flex_none()
                .w_3()
                .text_xs()
                .text_color(theme::subtle())
                .child(chevron),
        )
        .child(render_status_badge(&file.status))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_baseline()
                .overflow_hidden()
                .whitespace_nowrap()
                .children(renamed_from.map(|previous| {
                    div()
                        .flex_none()
                        .text_color(theme::subtle())
                        .child(format!("{previous} → "))
                }))
                .child(
                    div()
                        .flex_none()
                        .text_color(theme::muted())
                        .child(directory.to_owned()),
                )
                .child(
                    div()
                        .flex_none()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme::text())
                        .child(name.to_owned()),
                ),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .gap_1p5()
                .text_xs()
                .child(
                    div()
                        .text_color(theme::added())
                        .child(format!("+{}", file.additions)),
                )
                .child(
                    div()
                        .text_color(theme::removed())
                        .child(format!("−{}", file.deletions)),
                ),
        )
        .child(
            div()
                .id(("viewed", index))
                .flex_none()
                .flex()
                .items_center()
                .gap_1p5()
                .ml_1()
                .px_1p5()
                .py_0p5()
                .rounded_md()
                .text_xs()
                .text_color(if state.is_viewed {
                    theme::text()
                } else {
                    theme::muted()
                })
                .hover(|style| style.bg(theme::hover()))
                // The checkbox click must not also collapse the file.
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    on_toggle_viewed(event, window, cx);
                })
                .child(Checkbox::new(CheckState::from_bool(state.is_viewed)))
                .child("Viewed"),
        )
}

/// A path as its directory (with the trailing `/`) and its file name.
fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(slash) => path.split_at(slash + 1),
        None => ("", path),
    }
}

/// A one-letter badge for GitHub's file status, colored like GitHub's.
fn render_status_badge(status: &str) -> Div {
    let (letter, color): (&str, Rgba) = match status {
        "added" => ("A", theme::added()),
        "removed" => ("D", theme::removed()),
        "renamed" => ("R", theme::renamed()),
        "copied" => ("C", theme::renamed()),
        _ => ("M", theme::accent()),
    };
    div()
        .flex_none()
        .w(px(STATUS_BADGE_SIZE))
        .h(px(STATUS_BADGE_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.))
        .border_1()
        .border_color(color)
        .text_size(px(9.))
        .font_weight(FontWeight::BOLD)
        .text_color(color)
        .child(letter)
}
