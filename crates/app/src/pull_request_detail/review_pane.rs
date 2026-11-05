use gpui::{Entity, Subscription, Task};
use qc_cache::NoteDestination;

use super::comment_editor::CommentEditor;
use super::comment_placement::CommentPlacement;
use super::review_comments::{PlacedCard, ReviewComments};
use super::review_status::ReviewStatus;
use super::side_panel_tab::SidePanelTab;
use crate::text_area::TextArea;

/// The review side of the detail pane: the comments of the shown files and
/// their cards in the diff, where new comments go (GitHub or local), the
/// comment being written, the side panel's tab and the review summary.
pub struct ReviewPane {
    pub comments: ReviewComments,
    /// The cards shown for the active destination, in placement order.
    pub cards: Vec<PlacedCard>,
    pub placement: CommentPlacement,
    /// Where new comments go on a single pull request (combined reviews are always local).
    pub destination: NoteDestination,
    pub editor: Option<CommentEditor>,
    pub status: ReviewStatus,
    pub tab: SidePanelTab,
    /// The GitHub review's summary, while the review tab shows it.
    pub summary_editor: Option<Entity<TextArea>>,
    pub summary_subscription: Option<Subscription>,
    /// Reading GitHub's conversations.
    pub load_task: Option<Task<()>>,
    /// A submission, an edit or a deletion on GitHub.
    pub action_task: Option<Task<()>>,
}

impl Default for ReviewPane {
    fn default() -> Self {
        Self {
            comments: ReviewComments::default(),
            cards: Vec::new(),
            placement: CommentPlacement::default(),
            destination: NoteDestination::GitHub,
            editor: None,
            status: ReviewStatus::Idle,
            tab: SidePanelTab::Files,
            summary_editor: None,
            summary_subscription: None,
            load_task: None,
            action_task: None,
        }
    }
}
