use std::rc::Rc;

use gpui::{App, ClickEvent, Div, FontWeight, Stateful, Window, div, prelude::*};
use qd_github::PullRequestFile;

use super::FileDiffState;
use super::file_status_badge::render_file_status_badge;
use crate::checkbox::{CheckState, Checkbox};
use crate::fold_chevron::{FoldChevron, FoldState};
use crate::theme;
use crate::tree_locate_button::TreeLocateButton;

/// A click handler shared by several elements of one file.
pub type FileClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// The file's header row, the top of its card: chevron, status, path
/// (old → new for renames) with the button showing the file in the file
/// tree right after it, line counts and the viewed checkbox. Clicking the
/// row collapses or expands the file; clicking the button or the checkbox
/// does only what it says.
pub fn render_file_header(
    index: usize,
    file: &PullRequestFile,
    state: FileDiffState,
    on_toggle_collapsed: FileClickHandler,
    on_toggle_viewed: FileClickHandler,
    on_show_in_tree: FileClickHandler,
) -> Stateful<Div> {
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
        .child(FoldChevron::new(
            ("file-fold", index),
            FoldState::of(state.is_collapsed),
        ))
        .child(render_file_status_badge(&file.status))
        // The path, cut at the right when too long, with the tree button right after it.
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .gap_1()
                .child(render_file_path(file))
                .child(TreeLocateButton::new(
                    ("show-in-tree", index),
                    move |event, window, cx| on_show_in_tree(event, window, cx),
                )),
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

/// The file's path on one line, cut at the right when too long: the old
/// path for a rename, the directory muted and the name in full color.
fn render_file_path(file: &PullRequestFile) -> Div {
    let (directory, name) = split_path(&file.filename);
    let renamed_from = file
        .previous_filename
        .as_ref()
        .filter(|previous| **previous != file.filename);
    div()
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
        )
}

/// A path as its directory (with the trailing `/`) and its file name.
fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(slash) => path.split_at(slash + 1),
        None => ("", path),
    }
}
