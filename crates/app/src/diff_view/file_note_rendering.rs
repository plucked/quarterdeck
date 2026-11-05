use gpui::{AnyElement, IntoElement, div, prelude::*};

use super::FileNote;
use super::file_card_rendering::frame_file_row;
use super::file_header_rendering::FileClickHandler;
use crate::theme;

/// A file's note row; an expandable note calls `on_expand` when clicked.
pub fn render_file_note(index: usize, note: FileNote, on_expand: FileClickHandler) -> AnyElement {
    let row = frame_file_row(div().px_3().py_2().text_xs().child(note.message()));
    if note.is_expandable() {
        row.id(("file-note", index))
            .text_color(theme::accent())
            .cursor_pointer()
            .on_click(move |event, window, cx| on_expand(event, window, cx))
            .into_any_element()
    } else {
        row.text_color(theme::muted()).into_any_element()
    }
}
