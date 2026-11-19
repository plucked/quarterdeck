use std::rc::Rc;

use gpui::{Entity, Subscription, Task};

use super::diff_find_matches::DiffFindMatches;
use crate::diff_view::DiffMatchReveal;
use crate::text_area::TextArea;

/// An open Find bar for one file. Row positions are relative to that file's header.
pub(super) struct DiffFind {
    pub file: usize,
    pub input: Entity<TextArea>,
    pub matches: DiffFindMatches,
    pub current: usize,
    /// Consumed by the matching code cell once its exact wrapped position has been laid out.
    pub reveal: Rc<DiffMatchReveal>,
    pub task: Option<Task<()>>,
    /// The file's displayed rows changed and its results must be recomputed after the rebuild.
    pub rows_changed: bool,
    pub _subscriptions: [Subscription; 2],
}
