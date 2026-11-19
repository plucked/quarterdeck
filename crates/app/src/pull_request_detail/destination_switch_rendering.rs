use gpui::{App, Context, Window};
use qd_cache::NoteDestination;

use super::pull_request_detail::PullRequestDetail;
use crate::controls::SegmentedControl;

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
        let pick = |chosen| {
            let detail = detail.clone();
            move |window: &mut Window, cx: &mut App| {
                detail
                    .update(cx, |detail, cx| {
                        detail.set_comment_destination(chosen, cx);
                        if let Some(editor) = &detail.review().editor {
                            editor
                                .text
                                .update(cx, |text, cx| text.resume_writing(window, cx));
                        }
                    })
                    .ok();
            }
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
