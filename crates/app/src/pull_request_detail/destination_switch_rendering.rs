use gpui::Context;
use qd_cache::NoteDestination;

use super::pull_request_detail::PullRequestDetail;
use crate::controls::{SegmentedControl, view_option};

impl PullRequestDetail {
    /// The GitHub / Local switch of where new comments go on a single pull
    /// request (kept for the next comments and launches), its options
    /// labelled `github_label` and `local_label`.
    pub(super) fn render_destination_control(
        &self,
        id: &'static str,
        github_label: &'static str,
        local_label: &'static str,
        cx: &mut Context<Self>,
    ) -> SegmentedControl {
        let destination = self.review().destination;
        let detail = cx.entity().downgrade();
        let pick = |destination| {
            view_option(
                &detail,
                destination,
                PullRequestDetail::set_comment_destination,
            )
        };
        SegmentedControl::new(id)
            .option(
                github_label,
                destination == NoteDestination::GitHub,
                pick(NoteDestination::GitHub),
            )
            .option(
                local_label,
                destination == NoteDestination::Local,
                pick(NoteDestination::Local),
            )
    }
}
