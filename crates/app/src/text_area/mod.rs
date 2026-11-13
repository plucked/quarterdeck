//! `TextArea`, a plain-text editor, multiline or a field of one line: gpui
//! ships none.
//!
//! Files:
//! - `text_area.rs`: `TextArea`, the text, selection and editing, and `Render`.
//! - `line_mode.rs`: `LineMode`, multiline or one line: what Return does and
//!   the newlines dropped from inserted text.
//! - `text_area_actions.rs`: its actions, key bindings and key context.
//! - `text_area_event.rs`: `TextAreaEvent`, what it tells its owner.
//! - `text_area_input.rs`: the system text input (typing, input methods) for it.
//! - `text_area_element.rs`: the element that lays out, paints and hit-tests the text.
//! - `text_area_layout.rs`: `TextAreaLayout`, the shaped lines and offset ↔ position mapping.
//! - `text_boundaries.rs`: character, word and line boundaries in a text.

mod line_mode;
mod text_area;
mod text_area_actions;
mod text_area_element;
mod text_area_event;
mod text_area_input;
mod text_area_layout;
mod text_boundaries;

pub use text_area::TextArea;
pub use text_area_actions::bind_text_area_keys;
pub(crate) use text_area_actions::{Down as TextAreaCursorDown, Up as TextAreaCursorUp};
pub use text_area_event::TextAreaEvent;
