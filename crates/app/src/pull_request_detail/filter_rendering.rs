use gpui::{AnyElement, ClickEvent, Context, IntoElement, div, prelude::*};
use qd_file_tree::{FacetChoice, FacetCount};

use super::pull_request_detail::PullRequestDetail;
use crate::button::Button;
use crate::checkbox::{CheckState, Checkbox};
use crate::facet_chips::{
    COLLAPSED_CHIPS, ChipClick, most_common_values, render_chip, render_chip_row, render_more_chip,
};
use crate::theme;

impl PullRequestDetail {
    /// The filter under the progress bar: the viewed switch, the kind and
    /// type chip rows with live counts, and a line saying what is shown,
    /// right above the tree, with the button folding every folder beside it.
    pub(super) fn render_filter_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(self.render_hide_viewed_switch(cx))
            .child(self.render_kind_row(cx))
            .child(self.render_type_row(cx))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(self.render_filter_summary(cx)),
                    )
                    .children(self.render_fold_all_button(cx)),
            )
    }

    /// `☐ Hide viewed (12)`: hides viewed files on top of the chips.
    fn render_hide_viewed_switch(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_on = self.file_tree().filter().hide_viewed;
        let viewed = self.file_review().viewed_count();
        div()
            .id("hide-viewed")
            .flex()
            .items_center()
            .gap_1p5()
            .text_xs()
            .cursor_pointer()
            .text_color(theme::muted())
            .hover(|style| style.text_color(theme::text()))
            .on_click(cx.listener(|detail, _, _, cx| detail.toggle_hide_viewed(cx)))
            .child(Checkbox::new(CheckState::from_bool(is_on)))
            .child("Hide viewed")
            .child(div().text_color(theme::subtle()).child(viewed.to_string()))
    }

    /// `Collapse all` while a folder is expanded, else `Expand all`: folds
    /// or unfolds every folder of the tree at once. None for a flat list.
    fn render_fold_all_button(&self, cx: &mut Context<Self>) -> Option<Button> {
        let tree = self.file_tree();
        if !tree.has_folders() {
            return None;
        }
        let label = if tree.has_expanded_folder() {
            "Collapse all"
        } else {
            "Expand all"
        };
        Some(
            Button::new("fold-all-folders", label)
                .ghost()
                .on_click(cx.listener(|detail, _, _, cx| detail.toggle_all_folders(cx))),
        )
    }

    /// The kind chips: Code, Tests, Generated, Moved (those present).
    fn render_kind_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let filter = self.file_tree().filter();
        let chips: Vec<AnyElement> = self
            .file_tree()
            .kind_counts()
            .iter()
            .map(|count| {
                let kind = count.value;
                render_chip(
                    format!("kind-{}", kind.label()).into(),
                    kind.label(),
                    count,
                    filter.kinds.choice_of(&kind),
                    None,
                )
                .on_click(cx.listener(move |detail, event: &ClickEvent, _, cx| {
                    detail.click_kind(kind, ChipClick::of(event), cx);
                }))
                .into_any_element()
            })
            .collect();
        render_chip_row("Kind", chips)
    }

    /// The type chips in alphabetical order: collapsed, the most common
    /// ones and any picked or hidden; a "more" chip shows the rest.
    fn render_type_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let filter = self.file_tree().filter();
        let counts = self.file_tree().type_counts();
        let is_expanded = self.file_tree().are_types_expanded();

        // The most common, plus every chip that is set, so a set chip never hides behind "more".
        let most_common = most_common_values(counts);
        let shown: Vec<&FacetCount<String>> = counts
            .iter()
            .filter(|count| {
                is_expanded
                    || most_common.contains(&count.value.as_str())
                    || filter.file_types.choice_of(&count.value) != FacetChoice::Open
            })
            .collect();
        let hidden_count = counts.len() - shown.len();
        let mut chips: Vec<AnyElement> = shown
            .into_iter()
            .map(|count| {
                let file_type = count.value.clone();
                render_chip(
                    format!("type-{file_type}").into(),
                    &count.value,
                    count,
                    filter.file_types.choice_of(&count.value),
                    None,
                )
                .on_click(cx.listener(move |detail, event: &ClickEvent, _, cx| {
                    detail.click_file_type(&file_type, ChipClick::of(event), cx);
                }))
                .into_any_element()
            })
            .collect();

        // The more / less chip, when there is more than the collapsed row holds.
        let more_label = match (is_expanded, hidden_count) {
            (true, _) if counts.len() > COLLAPSED_CHIPS => Some("less".to_owned()),
            (false, 1..) => Some(format!("+{hidden_count} more")),
            (true | false, _) => None,
        };
        if let Some(label) = more_label {
            chips.push(
                render_more_chip("type-more", label)
                    .on_click(cx.listener(|detail, _, _, cx| detail.toggle_types_expanded(cx)))
                    .into_any_element(),
            );
        }
        render_chip_row("Type", chips)
    }

    /// `Showing 312 of 1390 · Moved · Reset` while anything narrows the
    /// files; otherwise how the chips work.
    fn render_filter_summary(&self, cx: &mut Context<Self>) -> AnyElement {
        let filter = self.file_tree().filter();
        let parts = filter.describe();
        if parts.is_empty() {
            return div()
                .text_xs()
                .text_color(theme::subtle())
                .child("Click to show only · ⌥-click to hide")
                .into_any_element();
        }
        let (shown, total) = self.file_tree().shown_of_total();
        div()
            .flex()
            .gap_1p5()
            .text_xs()
            .text_color(theme::muted())
            .child(div().flex_1().min_w_0().truncate().child(format!(
                "Showing {shown} of {total} · {}",
                parts.join(" · ")
            )))
            .when(filter.has_chips_set(), |summary| {
                summary.child(
                    div()
                        .id("filter-reset")
                        .flex_none()
                        .text_color(theme::accent())
                        .cursor_pointer()
                        .on_click(cx.listener(|detail, _, _, cx| detail.clear_filter_chips(cx)))
                        .child("Reset"),
                )
            })
            .into_any_element()
    }
}
