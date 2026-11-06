use gpui::{AnyElement, ClickEvent, Context, Div, IntoElement, div, prelude::*, px};
use qd_cache::{OthersFilter, UpdatedWithin};
use qd_file_tree::{FacetChoice, FacetCount};
use qd_github::RepoRef;

use super::ReviewQueue;
use super::others_filtering::{FilterContext, OthersFacet, chip_counts, describe};
use super::others_tab::review_requested_ids;
use crate::checkbox::{CheckState, Checkbox};
use crate::display_date::unix_now;
use crate::facet_chips::{
    COLLAPSED_CHIPS, ChipClick, render_chip, render_chip_row, render_more_chip,
};
use crate::segmented_control::SegmentedControl;
use crate::settings_window::{SettingsCategory, open_settings_window_at};
use crate::theme;

/// Size of the filter button on a repository's header.
const FILTER_BUTTON_SIZE: f32 = 24.;
/// The filter glyph: three bars, each shorter, centered.
const FILTER_GLYPH_BAR_WIDTHS: [f32; 3] = [12., 8., 4.];
const FILTER_GLYPH_BAR_HEIGHT: f32 = 1.5;

impl ReviewQueue {
    /// A repository's filter, open under its header in the Others tab: two
    /// switches, how recently updated, the label and author chips (click
    /// shows only those, ⌥-click hides) with live counts, then a line
    /// saying what narrows the list and a Reset.
    pub(super) fn render_others_filter(
        &self,
        repo: &RepoRef,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let filter = self.others_tab().filter(repo);
        let pull_requests: Vec<_> = self
            .lists()
            .others
            .iter()
            .filter(|pull_request| &pull_request.repo == repo)
            .cloned()
            .collect();
        let review_requested = review_requested_ids(self.lists());
        let context = FilterContext {
            review_requested: &review_requested,
            now: unix_now(),
        };
        let label_counts = chip_counts(&filter, OthersFacet::Labels, &pull_requests, &context);
        let author_counts = chip_counts(&filter, OthersFacet::Authors, &pull_requests, &context);

        div()
            .mx_2()
            .mt_0p5()
            .mb_1()
            .p_2p5()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .rounded_lg()
            .border_1()
            .border_color(theme::border())
            .bg(theme::surface())
            .child(Self::render_filter_switches(repo, &filter, cx))
            .child(Self::render_updated_within(repo, filter.updated_within, cx))
            .children((!label_counts.is_empty()).then(|| {
                self.render_filter_chips(repo, OthersFacet::Labels, &filter, &label_counts, cx)
            }))
            .child(self.render_filter_chips(
                repo,
                OthersFacet::Authors,
                &filter,
                &author_counts,
                cx,
            ))
            .child(Self::render_filter_summary(repo, &filter, cx))
            .into_any_element()
    }

    /// `☐ Hide drafts   ☐ Hide review requests`.
    fn render_filter_switches(
        repo: &RepoRef,
        filter: &OthersFilter,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let switch = |id: &'static str, label: &'static str, is_on: bool| {
            div()
                .id(id)
                .flex()
                .items_center()
                .gap_1p5()
                .text_xs()
                .cursor_pointer()
                .text_color(theme::muted())
                .hover(|style| style.text_color(theme::text()))
                .child(Checkbox::new(CheckState::from_bool(is_on)))
                .child(label)
        };
        let drafts_repo = repo.clone();
        let requests_repo = repo.clone();
        div()
            .flex()
            .flex_wrap()
            .gap_x_4()
            .gap_y_1()
            .child(
                switch("others-hide-drafts", "Hide drafts", filter.hide_drafts).on_click(
                    cx.listener(move |queue, _, _, cx| {
                        queue.change_others_filter(
                            &drafts_repo,
                            |filter| filter.hide_drafts = !filter.hide_drafts,
                            cx,
                        );
                    }),
                ),
            )
            .child(
                switch(
                    "others-hide-review-requests",
                    "Hide review requests",
                    filter.hide_review_requests,
                )
                .on_click(cx.listener(move |queue, _, _, cx| {
                    queue.change_others_filter(
                        &requests_repo,
                        |filter| filter.hide_review_requests = !filter.hide_review_requests,
                        cx,
                    );
                })),
            )
    }

    /// `UPDATED  Any time | 7 days | 30 days`.
    fn render_updated_within(
        repo: &RepoRef,
        updated_within: UpdatedWithin,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let queue = cx.entity().downgrade();
        let control = UpdatedWithin::ALL.into_iter().fold(
            SegmentedControl::new("others-updated-within"),
            |control, option| {
                let queue = queue.clone();
                let repo = repo.clone();
                control.option(
                    updated_within_label(option),
                    option == updated_within,
                    move |_, cx| {
                        queue
                            .update(cx, |queue, cx| {
                                queue.change_others_filter(
                                    &repo,
                                    |filter| filter.updated_within = option,
                                    cx,
                                );
                            })
                            .ok();
                    },
                )
            },
        );
        render_chip_row("Updated", vec![control.into_any_element()])
    }

    /// One chip row (labels or authors): collapsed, the most common chips
    /// and every picked or hidden one; `+n more` shows the rest.
    fn render_filter_chips(
        &self,
        repo: &RepoRef,
        facet: OthersFacet,
        filter: &OthersFilter,
        counts: &[FacetCount<String>],
        cx: &mut Context<Self>,
    ) -> Div {
        let chips_filter = match facet {
            OthersFacet::Labels => &filter.labels,
            OthersFacet::Authors => &filter.authors,
        };
        let is_expanded = self.others_tab().is_chip_row_expanded(repo, facet);

        // The first ones (most matching first), plus every chip that is set,
        // so a set chip never hides behind "more".
        let most_common: Vec<&str> = counts
            .iter()
            .take(COLLAPSED_CHIPS)
            .map(|count| count.value.as_str())
            .collect();
        let shown: Vec<&FacetCount<String>> = counts
            .iter()
            .filter(|count| {
                is_expanded
                    || most_common.contains(&count.value.as_str())
                    || chips_filter.choice_of(&count.value) != FacetChoice::Open
            })
            .collect();
        let hidden_count = counts.len() - shown.len();
        let mut chips: Vec<AnyElement> = shown
            .into_iter()
            .map(|count| {
                let value = count.value.clone();
                let repo = repo.clone();
                render_chip(
                    format!("others-{}-{value}", facet_id(facet)).into(),
                    &count.value,
                    count,
                    chips_filter.choice_of(&count.value),
                )
                .on_click(cx.listener(move |queue, event: &ClickEvent, _, cx| {
                    let click = ChipClick::of(event);
                    queue.change_others_filter(
                        &repo,
                        |filter| click_chip(filter, facet, &value, click),
                        cx,
                    );
                }))
                .into_any_element()
            })
            .collect();

        // The more / less link, when there is more than the collapsed row holds.
        let more_label = match (is_expanded, hidden_count) {
            (true, _) if counts.len() > COLLAPSED_CHIPS => Some("less".to_owned()),
            (false, 1..) => Some(format!("+{hidden_count} more")),
            (true | false, _) => None,
        };
        if let Some(label) = more_label {
            let repo = repo.clone();
            chips.push(
                render_more_chip(("others-more", facet_index(facet)), label)
                    .on_click(
                        cx.listener(move |queue, _, _, cx| queue.toggle_chip_row(&repo, facet, cx)),
                    )
                    .into_any_element(),
            );
        }
        render_chip_row(facet_label(facet), chips)
    }

    /// `Showing 4 of 12 · bug · no drafts   Reset` while the filter narrows
    /// anything, else how the chips work.
    fn render_filter_summary(
        repo: &RepoRef,
        filter: &OthersFilter,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let parts = describe(filter);
        let reset_repo = repo.clone();
        div()
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(theme::subtle())
                    .child(filter_summary_text(&parts)),
            )
            .children(filter.is_active().then(|| {
                div()
                    .id("others-filter-reset")
                    .flex_none()
                    .text_color(theme::accent())
                    .cursor_pointer()
                    .on_click(cx.listener(move |queue, _, _, cx| {
                        queue.change_others_filter(
                            &reset_repo,
                            |filter| *filter = OthersFilter::default(),
                            cx,
                        );
                    }))
                    .child("Reset")
            }))
    }

    /// The note under an Others repository that shows nothing: none open,
    /// or the filter hides them all (with a Reset).
    pub(super) fn render_nothing_shown(
        &self,
        repo: &RepoRef,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let total = self
            .lists()
            .others
            .iter()
            .filter(|pull_request| &pull_request.repo == repo)
            .count();
        let note = div()
            .px_4()
            .py_1p5()
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .text_color(theme::subtle());
        if total == 0 {
            return note
                .child("No open pull requests by others")
                .into_any_element();
        }
        let reset_repo = repo.clone();
        note.child(
            div()
                .flex_1()
                .child(format!("The filter hides all {total}")),
        )
        .child(
            div()
                .id("others-nothing-reset")
                .text_color(theme::accent())
                .cursor_pointer()
                .on_click(cx.listener(move |queue, _, _, cx| {
                    queue.change_others_filter(
                        &reset_repo,
                        |filter| *filter = OthersFilter::default(),
                        cx,
                    );
                }))
                .child("Reset filter"),
        )
        .into_any_element()
    }
}

/// The Others tab with no watched repository: what it is for, and the way
/// to choose some.
pub(super) fn render_others_empty_state() -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_3()
        .px_6()
        .pt_10()
        .text_center()
        .child(
            div()
                .text_sm()
                .text_color(theme::text())
                .child("Follow other people's work"),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme::muted())
                .child("Choose repositories, and their open pull requests by others show here, with filters per repository."),
        )
        .child(
            crate::button::Button::new("choose-repositories", "Choose repositories…")
                .primary()
                .on_click(|_, _, cx| open_settings_window_at(SettingsCategory::Repositories, cx)),
        )
}

/// The filter button on an Others repository's header: three bars, tinted
/// when the filter narrows the list, filled while it is open. Its click
/// does not fold the group.
pub(super) fn render_filter_button(
    row_index: usize,
    repo: &RepoRef,
    is_active: bool,
    is_open: bool,
    cx: &mut Context<ReviewQueue>,
) -> impl IntoElement {
    let repo = repo.clone();
    let bar_color = if is_active {
        theme::accent()
    } else {
        theme::muted()
    };
    div()
        .id(("others-filter-button", row_index))
        .flex_none()
        .size(px(FILTER_BUTTON_SIZE))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(2.))
        .rounded_md()
        .cursor_pointer()
        .when(is_open, |button| button.bg(theme::selected()))
        .hover(|style| style.bg(theme::hover()))
        .on_click(cx.listener(move |queue, _, _, cx| {
            cx.stop_propagation();
            queue.toggle_others_filter_panel(&repo, cx);
        }))
        .children(FILTER_GLYPH_BAR_WIDTHS.map(|width| {
            div()
                .w(px(width))
                .h(px(FILTER_GLYPH_BAR_HEIGHT))
                .rounded_full()
                .bg(bar_color)
        }))
}

/// What narrows the list, or how the chips work while nothing does.
fn filter_summary_text(parts: &[String]) -> String {
    if parts.is_empty() {
        "Click to show only · ⌥-click to hide".to_owned()
    } else {
        parts.join(" · ")
    }
}

/// Applies a chip click to `filter`'s row of `facet`.
fn click_chip(filter: &mut OthersFilter, facet: OthersFacet, value: &String, click: ChipClick) {
    let chips = match facet {
        OthersFacet::Labels => &mut filter.labels,
        OthersFacet::Authors => &mut filter.authors,
    };
    match click {
        ChipClick::Pick => chips.toggle_only(value),
        ChipClick::Hide => chips.toggle_hidden(value),
    }
}

fn updated_within_label(updated_within: UpdatedWithin) -> &'static str {
    match updated_within {
        UpdatedWithin::AnyTime => "Any time",
        UpdatedWithin::Week => "7 days",
        UpdatedWithin::Month => "30 days",
    }
}

fn facet_label(facet: OthersFacet) -> &'static str {
    match facet {
        OthersFacet::Labels => "Labels",
        OthersFacet::Authors => "Authors",
    }
}

/// A short name of the facet, for element ids.
fn facet_id(facet: OthersFacet) -> &'static str {
    match facet {
        OthersFacet::Labels => "label",
        OthersFacet::Authors => "author",
    }
}

fn facet_index(facet: OthersFacet) -> usize {
    match facet {
        OthersFacet::Labels => 0,
        OthersFacet::Authors => 1,
    }
}
