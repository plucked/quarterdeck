//! Choosing an existing local Claude session for a one-way review submission.
//!
//! - `claude_session_picker.rs`: the view, discovery and submission tasks, keyboard selection.
//! - `session_picker_phase.rs`: discovery and submission progress and feedback.
//! - `session_picker_rendering.rs`: the virtualized destination list and send controls.
//! - `session_presentation.rs`: concise row labels and full details for search and hover.
//! - `session_group.rs`: repository grouping with separate identities for same-named clones.
//! - `session_picker_row.rs`: non-selectable headings and filtered session rows.
//! - `session_list_state.rs`: virtualized scroll configuration and native wheel regression.
//! - `session_selection.rs`: filtering and explicit selection within the visible matches.
//! - `session_checkout_loading.rs`: bounded background repository and branch lookups after the list appears.

mod claude_session_picker;
mod session_checkout_loading;
mod session_group;
mod session_list_state;
mod session_picker_phase;
mod session_picker_rendering;
mod session_picker_row;
mod session_presentation;
mod session_selection;

pub use claude_session_picker::ClaudeSessionPicker;
