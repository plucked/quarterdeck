use gpui::{AnyElement, Context, IntoElement, div, prelude::*, px, svg};
use qd_cache::FilesMode;

use super::pull_request_detail::PullRequestDetail;
use super::search_scope::SearchScope;
use crate::controls::{Button, NumberFlow, ProgressBar, TextTooltip};
use crate::theme;

const COMPACT_RESET_WIDTH: f32 = 350.;

impl PullRequestDetail {
    /// The approved Files shell: two control rows, optional filters, navigation and a quiet footer.
    pub(super) fn render_focus_drawer(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(self.render_focus_header(cx))
            .when(self.focus_drawer.filters_open, |pane| {
                pane.child(self.render_filter_controls(cx))
            })
            .child(match self.focus_drawer.mode {
                FilesMode::Tree if !self.file_tree.search.query.trim().is_empty() => {
                    self.render_search_results(cx)
                }
                FilesMode::Tree => self.render_tree_body(cx),
                FilesMode::Plan
                    if self.brief().mode == super::detail_mode::DetailMode::GuidedReview
                        && self
                            .current_smart_filter()
                            .is_none_or(|state| state.filter.is_none()) =>
                {
                    Self::render_guided_navigation_placeholder()
                }
                FilesMode::Plan => self.render_walkthrough(cx),
            })
            .child(self.render_files_footer(cx))
            .into_any_element()
    }

    /// Scoped search and Filters above the combined horizontal progress control and viewed actions.
    fn render_focus_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .pt_1()
            .pb_3()
            .border_b_1()
            .border_color(theme::border())
            .child(self.render_files_search_row(cx))
            .child(self.render_viewed_row(cx))
    }

    /// The first header row keeps scoped search beside the filter drawer toggle.
    fn render_files_search_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let scope = self.file_tree.search.scope;
        let next_scope = match scope {
            SearchScope::Paths => SearchScope::Contents,
            SearchScope::Contents => SearchScope::Paths,
        };
        div()
            .flex()
            .items_center()
            .gap_1()
            .min_w_0()
            .child(
                div()
                    .id("search-scope-hint")
                    .tooltip(TextTooltip::builder(format!(
                        "Search {}. Click to search {}.",
                        scope.label().to_lowercase(),
                        next_scope.label().to_lowercase()
                    )))
                    .child(Button::new("search-scope", scope.label()).ghost().on_click(
                        cx.listener(move |detail, _, _, cx| {
                            detail.set_file_search_scope(next_scope, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(self.focus_drawer.input.clone()),
            )
            .child(
                div()
                    .id("files-filters-toggle")
                    .flex_none()
                    .size(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .cursor_pointer()
                    .tooltip(TextTooltip::builder("Filters: Kind and Type"))
                    .when(
                        self.focus_drawer.filters_open || self.file_tree().has_chips_set(),
                        |button| button.bg(theme::selected()),
                    )
                    .hover(|style| style.bg(theme::hover()))
                    .on_click(cx.listener(|detail, _, _, cx| detail.toggle_filters_drawer(cx)))
                    .child(
                        svg()
                            .path("icons/filter.svg")
                            .size(px(14.))
                            .text_color(theme::muted()),
                    ),
            )
    }

    /// The second header row combines progress with the viewed controls.
    fn render_viewed_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let viewed = self.file_review().viewed_count();
        let total = self.loaded_files().len();
        let fraction = f32::from(u16::try_from(viewed).unwrap_or(u16::MAX))
            / f32::from(u16::try_from(total.max(1)).unwrap_or(u16::MAX));
        div()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h(px(26.))
                    .flex()
                    .flex_col()
                    .justify_between()
                    .rounded_md()
                    .overflow_hidden()
                    .border_1()
                    .border_color(theme::border())
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(11.))
                            .child(NumberFlow::new(
                                "viewed-count",
                                format!("{viewed}/{total} viewed"),
                            )),
                    )
                    .child(ProgressBar::new(
                        "files-viewed-progress",
                        fraction,
                        theme::border(),
                        theme::success(),
                    )),
            )
            .child(self.render_hide_viewed_switch(cx))
            .child(
                div()
                    .id("reset-viewed")
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_1()
                    .p_1()
                    .rounded_md()
                    .tooltip(TextTooltip::builder(
                        "Reset viewed for every loaded file, including filtered files",
                    ))
                    .when(viewed == 0, |button| button.opacity(0.4))
                    .when(viewed > 0, |button| {
                        button
                            .cursor_pointer()
                            .hover(|style| style.bg(theme::hover()))
                            .on_click(cx.listener(|detail, _, _, cx| {
                                detail.reset_viewed_files(cx);
                            }))
                    })
                    .child(
                        svg()
                            .path("icons/reset.svg")
                            .size(px(14.))
                            .text_color(theme::muted()),
                    )
                    .when(self.side_panel_width() >= COMPACT_RESET_WIDTH, |button| {
                        button.child(
                            div()
                                .text_size(px(11.))
                                .text_color(theme::muted())
                                .child("Reset viewed"),
                        )
                    }),
            )
    }

    /// Counts, folding, content coverage and Undo never consume a third header row.
    fn render_files_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (shown, total) = self.file_tree().shown_of_total();
        let compact = self.side_panel_width() < 300.;
        let count_label = match (compact, shown == total) {
            (true, true) => format!("{shown} files"),
            (true, false) => format!("{shown}/{total}"),
            (false, _) => format!("{shown}/{total} files"),
        };
        div()
            .flex_none()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(theme::border())
            .flex()
            .flex_col()
            .gap_1()
            .text_xs()
            .text_color(theme::subtle())
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("files-count-hint")
                            .flex_1()
                            .min_w_0()
                            .tooltip(TextTooltip::builder(format!(
                                "{shown} of {total} changed files match the current filters"
                            )))
                            .child(NumberFlow::label(count_label)),
                    )
                    .when(!self.viewed_undo.is_empty(), |row| {
                        row.child(
                            Button::new(
                                "undo-viewed",
                                if compact { "Undo" } else { "Undo viewed" },
                            )
                            .ghost()
                            .on_click(cx.listener(|detail, _, _, cx| detail.undo_viewed_files(cx))),
                        )
                    })
                    .when(
                        self.focus_drawer.mode == FilesMode::Tree
                            && self.file_tree.search.query.trim().is_empty(),
                        |row| row.children(self.render_fold_all_button(cx)),
                    ),
            )
            .when(
                self.file_tree.search.scope == SearchScope::Contents
                    && self.focus_drawer.content_started,
                |footer| footer.child(self.render_search_coverage(cx)),
            )
    }

    /// Partial results stay explicitly partial; Retry waits for a throttling deadline.
    fn render_search_coverage(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let drawer = &self.focus_drawer;
        let checked = drawer.indexed + drawer.skipped + drawer.failed;
        let total = self.loaded_files().len();
        let loading = drawer.content_task.is_some();
        let incomplete = checked < total || drawer.failed > 0;
        let status = if loading {
            "Loading"
        } else if incomplete {
            "Partial results"
        } else {
            "Searched"
        };
        div().id("search-coverage").flex().flex_col().gap_1()
            .tooltip(TextTooltip::builder(drawer.content_error.clone().unwrap_or_else(|| "Complete file contents at the loaded revision; binary files and files over 8 MB are unavailable.".into())))
            .child(div().flex().items_center().gap_1()
                .child(div().flex_1().min_w_0().child(NumberFlow::label(format!("{status} · {checked}/{total} files"))))
                .when(!loading && incomplete, |row| row.child(
                    Button::new("retry-content-search", "Retry").ghost().disabled(drawer.retry_task.is_some())
                        .on_click(cx.listener(|detail, _, _, cx| detail.retry_content_search(cx))))))
            .child(div().child(NumberFlow::label(format!("{} text · {} unavailable · {} failed", drawer.indexed, drawer.skipped, drawer.failed))))
            .when(drawer.retry_task.is_some(), |footer| footer.child("GitHub asked to wait before retrying."))
    }
}
