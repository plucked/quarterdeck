use gpui::{Entity, SharedString};

use crate::text_area::TextArea;

/// Where reporting the shown brief stands: closed (a button), being
/// written (the reader's words in a text field), being saved, saved (the
/// report's path, to copy into a Claude Code session), or failed (why,
/// with the words kept to try again).
#[derive(Default)]
pub enum ReportState {
    #[default]
    Closed,
    Writing(Entity<TextArea>),
    Saving,
    Saved(SharedString),
    Failed {
        message: SharedString,
        editor: Entity<TextArea>,
    },
}
