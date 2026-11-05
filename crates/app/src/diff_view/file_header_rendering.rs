use std::rc::Rc;

use gpui::{App, ClickEvent, Div, FontWeight, Rgba, Stateful, Window, div, prelude::*};
use qc_github::PullRequestFile;

use super::FileDiffState;
use crate::theme;

/// A click handler shared by several elements of one file.
pub type FileClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The file's header row: chevron, status, path (old → new for renames),
/// line counts and the viewed checkbox. Clicking the row collapses or
/// expands the file; clicking the checkbox only toggles viewed.
pub fn render_file_header(
    index: usize,
    file: &PullRequestFile,
    state: FileDiffState,
    on_toggle_collapsed: FileClickHandler,
    on_toggle_viewed: FileClickHandler,
) -> Stateful<Div> {
    let chevron = if state.is_collapsed { "▸" } else { "▾" };
    let path = match &file.previous_filename {
        Some(previous) if *previous != file.filename => format!("{previous} → {}", file.filename),
        Some(_) | None => file.filename.clone(),
    };
    let checkbox = if state.is_viewed {
        "☑ Viewed"
    } else {
        "☐ Viewed"
    };

    div()
        .id(("file-header", index))
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .bg(theme::panel())
        .cursor_pointer()
        .on_click(move |event, window, cx| on_toggle_collapsed(event, window, cx))
        .child(
            div()
                .flex_none()
                .w_3()
                .text_color(theme::muted())
                .child(chevron),
        )
        .child(render_status_badge(&file.status))
        .child(div().flex_1().min_w_0().truncate().child(path))
        .child(
            div()
                .flex_none()
                .text_color(theme::added())
                .child(format!("+{}", file.additions)),
        )
        .child(
            div()
                .flex_none()
                .text_color(theme::removed())
                .child(format!("−{}", file.deletions)),
        )
        .child(
            div()
                .id(("viewed", index))
                .flex_none()
                .px_2()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .text_xs()
                .when(state.is_viewed, |checkbox| checkbox.bg(theme::selected()))
                .when(!state.is_viewed, |checkbox| {
                    checkbox
                        .text_color(theme::muted())
                        .hover(|style| style.bg(theme::hover()))
                })
                // The checkbox click must not also collapse the file.
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    on_toggle_viewed(event, window, cx);
                })
                .child(checkbox),
        )
}

/// A one-letter badge for GitHub's file status, colored like GitHub's.
fn render_status_badge(status: &str) -> Div {
    let (letter, color): (&str, Rgba) = match status {
        "added" => ("A", theme::added()),
        "removed" => ("D", theme::removed()),
        "renamed" => ("R", theme::renamed()),
        "copied" => ("C", theme::renamed()),
        _ => ("M", theme::muted()),
    };
    div()
        .flex_none()
        .w_4()
        .text_center()
        .font_weight(FontWeight::BOLD)
        .text_color(color)
        .child(letter)
}
