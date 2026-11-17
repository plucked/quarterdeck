use gpui::{AnyElement, Context, IntoElement, div, prelude::*};

use super::file_tree_rendering::render_file_icon;
use super::pull_request_detail::PullRequestDetail;
use super::walkthrough_item_id::WalkthroughItemId;
use super::walkthrough_item_rendering::render_group_icon;
use crate::controls::{Button, NumberFlow};
use crate::display_text::counted;
use crate::theme;

impl PullRequestDetail {
    /// Every guided step keeps its note and ordered files together in the same diff workspace.
    pub(super) fn render_group_overview(
        &mut self,
        id: WalkthroughItemId,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.walkthrough.item(id) else {
            return div().into_any_element();
        };
        let bucket = &self.walkthrough.buckets[id.bucket];
        let icon = match item.files.as_slice() {
            [file] => render_file_icon(self.loaded_files()[*file].facets.icon, item.viewed == 1)
                .into_any_element(),
            _ => render_group_icon().into_any_element(),
        };
        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .p_4()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(div().flex_none().pt_1().child(icon))
                    .child(div().flex_1().min_w_0().text_lg().child(item.label.clone())),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme::muted())
                    .child(NumberFlow::label(format!(
                        "Step {} of {} in {} · {} · {} viewed",
                        id.item + 1,
                        bucket.items.len(),
                        bucket.label,
                        counted(item.files.len(), "file", "files"),
                        item.viewed
                    ))),
            )
            .when(!item.description.is_empty(), |view| {
                view.child(
                    div()
                        .min_w_0()
                        .text_sm()
                        .text_color(theme::muted())
                        .child(item.description.clone()),
                )
            })
            .child(
                div()
                    .flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new(
                            "overview-mark",
                            viewed_action_label(item.files.len(), item.viewed),
                        )
                        .on_click(cx.listener(move |detail, _, _, cx| {
                            detail.mark_walkthrough_viewed_and_advance(id, cx);
                        })),
                    )
                    .child(
                        Button::new("overview-next", "Next in order")
                            .ghost()
                            .disabled(self.walkthrough.next_after(id).is_none())
                            .on_click(cx.listener(move |detail, _, _, cx| {
                                detail.advance_walkthrough(id, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(theme::muted())
                    .child("Files follow the suggested reading order. Marking a file viewed collapses its diff."),
            )
            .child(self.render_diff_area(cx))
            .into_any_element()
    }
}

/// Keeps the viewed action explicit for both individual files and grouped steps.
fn viewed_action_label(files: usize, viewed: usize) -> &'static str {
    match (files, viewed == files) {
        (1, true) => "Unmark file",
        (1, false) => "Mark file viewed",
        (_, true) => "Unmark group",
        (_, false) => "Mark group viewed",
    }
}
