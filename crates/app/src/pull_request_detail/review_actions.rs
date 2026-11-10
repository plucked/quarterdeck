use qd_cache::NoteDestination;

use super::comment_editor::EditorTarget;
use super::comment_placement::CommentPlacement;
use super::comment_scope::CommentScope;
use super::diff_line_ref::DiffLineRef;
use super::operation_status::OperationStatus;
use super::pull_request_detail::PullRequestDetail;
use super::review_comments::ReviewComments;
use super::review_target::ReviewTarget;

impl PullRequestDetail {
    /// Where new comments go now: the chosen destination for a single pull
    /// request; always local for a combined review (GitHub has no place for it).
    pub(super) fn active_destination(&self) -> NoteDestination {
        match self.target() {
            ReviewTarget::Layer => self.review().destination,
            ReviewTarget::Combined(_) | ReviewTarget::AwaitingStack => NoteDestination::Local,
        }
    }

    /// Which comments the diff shows for the shown target.
    fn comment_scope(&self) -> CommentScope {
        match self.target() {
            ReviewTarget::Layer => CommentScope::GitHubAndLocal,
            ReviewTarget::Combined(_) | ReviewTarget::AwaitingStack => CommentScope::LocalOnly,
        }
    }

    /// Recomputes the cards for the active destination and where they and
    /// the composer go; the row rebuild that follows places them.
    pub(super) fn refresh_comment_layout(&mut self) {
        let composer = match self.review().editor.as_ref().map(|editor| &editor.target) {
            Some(EditorTarget::NewComment {
                file, side, line, ..
            }) => Some(DiffLineRef {
                file: *file,
                side: *side,
                line: *line,
            }),
            Some(_) | None => None,
        };
        let files: Vec<String> = self
            .loaded_files()
            .iter()
            .map(|loaded| loaded.filename().to_owned())
            .collect();
        let scope = self.comment_scope();
        let review = self.review_mut();
        review.comments.refresh_documents();
        review.cards = review.comments.cards(scope);
        review.placement = CommentPlacement::build(
            &review.cards,
            |path| files.iter().position(|file| file == path),
            composer,
        );
    }

    /// Starts over for newly loaded files: no comments, no editor.
    pub(super) fn reset_review(&mut self) {
        let review = self.review_mut();
        review.comments = ReviewComments::default();
        review.editor = None;
        review.load_task = None;
        review.action_task = None;
        review.status = OperationStatus::Idle;
        review.drop_summary_editor();
    }
}
