use std::rc::Rc;

use gpui::{App, Div, MouseButton, SharedString, Window, div, prelude::*};
use qd_diff::{ContextGap, GapPlace};

use super::diff_cell_rendering::render_row;
use super::file_card_rendering::frame_file_row;
use super::{CONTEXT_STEP, FileTextLoad, GapPart};
use crate::theme;

/// The most buttons a gap bar shows; ids of a row's buttons are spaced by it.
const MAX_GAP_BUTTONS: usize = 3;

/// What a click on a gap's button does: reveals that part of the gap.
pub type GapClickHandler = Rc<dyn Fn(&GapPart, &mut Window, &mut App)>;

/// One button of a gap bar: the part it reveals and its label.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GapButton {
    part: GapPart,
    label: String,
}

/// The bar over hidden lines inside a file's card: buttons revealing the
/// lines below the hunk above, above the hunk below, or all of them,
/// then `header` (the hunk's `@@` line, when the bar sits above a hunk).
/// While the file's text is fetched it says so; a failure shows beside the
/// buttons, which try again. `row` is the bar's row in the list, which
/// keeps its buttons' ids unique.
pub fn render_gap_bar(
    row: usize,
    gap: &ContextGap,
    header: Option<&str>,
    load: Option<&FileTextLoad>,
    on_click: &GapClickHandler,
) -> Div {
    let bar = render_row(None)
        .bg(theme::hunk_background())
        .pl_1()
        .gap_1()
        .items_center()
        .whitespace_nowrap()
        .overflow_hidden()
        .text_color(theme::subtle());
    let buttons = || {
        gap_buttons(gap)
            .into_iter()
            .enumerate()
            .map(|(position, button)| {
                render_gap_button(row * MAX_GAP_BUTTONS + position, button, on_click)
            })
    };
    let bar = match load {
        Some(FileTextLoad::Loading) => bar.child(div().px_1().child("Loading the file…")),
        Some(FileTextLoad::Failed(message)) => {
            bar.children(buttons())
                .child(
                    div()
                        .px_1()
                        .text_color(theme::danger())
                        .child(SharedString::from(format!(
                            "Can't show the lines: {message}"
                        ))),
                )
        }
        None => bar.children(buttons()),
    };
    frame_file_row(bar.children(header.map(|header| div().pl_2().child(header.to_owned()))))
}

/// One clickable label of a gap bar.
fn render_gap_button(id: usize, button: GapButton, on_click: &GapClickHandler) -> impl IntoElement {
    let on_click = on_click.clone();
    let part = button.part;
    div()
        .id(("gap-button", id))
        .flex_none()
        .px_1p5()
        .rounded_sm()
        .text_color(theme::accent())
        .cursor_pointer()
        .hover(|style| style.bg(theme::hover()))
        // A press here is a click, not the start of a code selection.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(move |_, window, cx| on_click(&part, window, cx))
        .child(button.label)
}

/// The buttons a gap offers: a short gap shows whole in one click; a long
/// one grows by [`CONTEXT_STEP`] lines from either hunk, or shows whole.
/// The top of the file grows only upward from the first hunk, the end only
/// downward from the last.
fn gap_buttons(gap: &ContextGap) -> Vec<GapButton> {
    let button = |part, label: String| GapButton { part, label };
    let step = CONTEXT_STEP;
    match (gap.place, gap.line_count()) {
        (_, Some(count)) if count <= step => {
            vec![button(
                GapPart::All,
                format!("Show {count} hidden {}", lines_word(count)),
            )]
        }
        (GapPlace::AboveHunk(0), Some(count)) => vec![
            button(GapPart::Bottom, format!("↑ {step} lines")),
            button(GapPart::All, format!("Show all {count}")),
        ],
        (GapPlace::AboveHunk(_), Some(count)) => vec![
            button(GapPart::Top, format!("↓ {step} lines")),
            button(GapPart::Bottom, format!("↑ {step} lines")),
            button(GapPart::All, format!("Show all {count}")),
        ],
        (GapPlace::End, Some(count)) => vec![
            button(GapPart::Top, format!("↓ {step} lines")),
            button(GapPart::All, format!("Show all {count}")),
        ],
        // Only the end can be of unknown length, before the text is fetched.
        (GapPlace::AboveHunk(_) | GapPlace::End, None) => vec![
            button(GapPart::Top, format!("↓ {step} lines")),
            button(GapPart::All, "Show the rest of the file".to_owned()),
        ],
    }
}

/// "line" or "lines" for `count`.
fn lines_word(count: u32) -> &'static str {
    if count == 1 { "line" } else { "lines" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(place: GapPlace, count: Option<u32>) -> ContextGap {
        ContextGap {
            place,
            new_start: 10,
            new_end: count.map(|count| 10 + count),
            old_offset: 0,
        }
    }

    fn parts(gap: &ContextGap) -> Vec<GapPart> {
        gap_buttons(gap)
            .into_iter()
            .map(|button| button.part)
            .collect()
    }

    /// A short gap shows whole; a long one between hunks grows either way.
    #[test]
    fn offers_parts_by_gap_length_and_place() {
        assert_eq!(parts(&gap(GapPlace::AboveHunk(1), Some(5))), [GapPart::All]);
        assert_eq!(
            gap_buttons(&gap(GapPlace::AboveHunk(1), Some(1)))[0].label,
            "Show 1 hidden line"
        );
        assert_eq!(
            parts(&gap(GapPlace::AboveHunk(1), Some(100))),
            [GapPart::Top, GapPart::Bottom, GapPart::All]
        );
        assert_eq!(
            parts(&gap(GapPlace::AboveHunk(0), Some(100))),
            [GapPart::Bottom, GapPart::All]
        );
        assert_eq!(
            parts(&gap(GapPlace::End, None)),
            [GapPart::Top, GapPart::All]
        );
    }
}
