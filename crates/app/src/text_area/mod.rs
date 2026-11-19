//! Text input for single-line fields, multiline notes and GitHub review composers.
//!
//! Files:
//! - `text_area.rs`: `TextArea`, the text, selection and editing, and `Render`.
//! - `comment_history.rs`: bounded undo and redo of comment edits and formatting.
//! - `line_mode.rs`: `LineMode`, multiline or one line: what Return does and
//!   the newlines dropped from inserted text.
//! - `text_area_actions.rs`: its actions, key bindings and key context.
//! - `text_area_event.rs`: `TextAreaEvent`, what it tells its owner.
//! - `text_area_input.rs`: the system text input (typing, input methods) for it.
//! - `text_area_element.rs`: the element that lays out, paints and hit-tests the text.
//! - `text_area_layout.rs`: `TextAreaLayout`, the shaped lines and offset ↔ position mapping.
//! - `text_boundaries.rs`: character, word and line boundaries in a text.
//! - `markdown_editor.rs`: a text area with GitHub Markdown formatting buttons.
//! - `markdown_format.rs`: selection and caret edits for the formatting buttons.
//! - `list_continuation.rs`: Return splits a rich list item or exits an empty one.
//! - `rich_blocks.rs`: toggles paragraph, list and code block formatting.
//! - `rich_line_runs.rs`: maps the Markdown projection’s styled runs to GPUI text runs.
//! - `rich_editing.rs`: selection, formatting, clipboard and typing in the rich text mode.
//! - `rich_inline_selection.rs`: merges and splits selected inline styles.
//! - `rich_link.rs`: editing a link address while preserving the label.
//! - `tests/`: keyboard and toolbar scenarios in a rendered composer.
//! - `suggestion_block.rs`: inserts an editable GitHub suggestion into a review draft.

#[cfg(test)]
mod tests;

mod comment_history;
mod line_mode;
mod list_continuation;
mod markdown_editor;
mod markdown_format;
mod rich_blocks;
mod rich_editing;
mod rich_inline_selection;
mod rich_line_runs;
mod rich_link;
mod suggestion_block;
mod text_area;
mod text_area_actions;
mod text_area_element;
mod text_area_event;
mod text_area_input;
mod text_area_layout;
mod text_boundaries;

pub use markdown_editor::MarkdownEditor;
pub use text_area::TextArea;
pub use text_area_actions::bind_text_area_keys;
pub(crate) use text_area_actions::{
    Down as TextAreaCursorDown, RedoCommentEdit, SelectAll as TextAreaSelectAll, UndoCommentEdit,
    Up as TextAreaCursorUp,
};
pub use text_area_event::TextAreaEvent;
