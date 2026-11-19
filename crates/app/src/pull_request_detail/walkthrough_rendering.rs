use gpui::{AnyElement, Context, IntoElement, div, list, prelude::*, px};
use qd_studio::AgentProgress;

use super::pull_request_detail::PullRequestDetail;
use super::walkthrough_row::WalkthroughRow;
use crate::controls::{Button, FoldChevron, FoldState, NumberFlow, PulsingStatus, TextTooltip};
use crate::display_text::counted;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

impl PullRequestDetail {
    /// Empty, generating, stale and ready states share the same virtualized walkthrough.
    pub(super) fn render_walkthrough(&self, cx: &mut Context<Self>) -> AnyElement {
        let state = self.current_smart_filter();
        let plan = state.and_then(|state| state.filter.as_ref());
        let running = state.is_some_and(|state| state.run.is_some() || state.saving);
        let pending_save = state.is_some_and(|state| state.pending_save.is_some()) && !running;
        let is_outdated = plan.is_some_and(|plan| self.smart_filter_is_stale(plan));
        let action = if plan.is_none() {
            "Create guided review"
        } else {
            "Refresh guide"
        };
        let mut area = div().flex_1().min_h_0().flex().flex_col();
        if plan.is_none() {
            area = area.child(div().p_4().pt_8().flex().flex_col().gap_3()
                .child(div().text_base().child("No guided review yet"))
                .child(div().text_xs().text_color(theme::muted()).child("Turn the changed files into focused buckets, with a suggested reading order and related assets grouped into one stop."))
                .child(Button::new("create-smart-plan",action).primary().disabled(running).on_click(cx.listener(|detail,_,_,cx| detail.generate_smart_filter(cx))))
                .child(Button::new("browse-tree","Browse file tree").ghost().on_click(cx.listener(|detail,_,_,cx| detail.select_detail_mode(super::detail_mode::DetailMode::Diff,cx))))
                .child(div().text_xs().text_color(theme::subtle()).child("Your viewed files stay marked when you create a guided review.")));
        } else {
            let unplanned = self
                .walkthrough
                .buckets
                .iter()
                .find(|bucket| bucket.order == 0)
                .map_or(0, |bucket| bucket.files.len());
            area = area.child(div().flex_none().px_3().py_2().flex().flex_col().gap_1p5().border_b_1().border_color(theme::border())
                .child(div().flex().items_center().gap_2().child(div().flex_1().min_w_0().text_xs().text_color(if is_outdated {theme::removed()} else {theme::subtle()}).child(if is_outdated {"Guide is outdated"} else {"Suggested walkthrough"}))
                    .child(Button::new("refresh-smart-plan",action).ghost().disabled(running).on_click(cx.listener(|detail,_,_,cx| detail.generate_smart_filter(cx)))))
                .when(is_outdated, |notice| notice.child(div().text_xs().text_color(theme::muted()).child("The PR changed. You can keep reviewing the saved guide; refresh to update its order and groups.")))
                .when(unplanned > 0, |notice| notice.child(Button::new("unplanned-files",format!("{unplanned} files not in the guide")).ghost().on_click(cx.listener(|detail,_,_,cx| detail.show_unplanned_files(cx))))));
        }
        if running {
            let progress = match state {
                Some(state) if state.saving => "Saving guided review…".to_owned(),
                _ => progress_label(state.and_then(|state| state.progress)),
            };
            area =
                area.child(
                    div()
                        .px_3()
                        .py_2()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_2()
                        .child(PulsingStatus::new("plan-generating", progress))
                        .child(Button::new("cancel-plan", "Cancel").ghost().on_click(
                            cx.listener(|detail, _, _, cx| detail.cancel_smart_filter(cx)),
                        )),
                );
        }
        // Keep the generated guide available for a save retry or an explicit discard.
        if let Some(error) = state.and_then(|state| state.error.as_ref()) {
            area = area.child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(theme::danger())
                    .child(error.clone()),
            );
        }
        if pending_save {
            area = area.child(Self::render_walkthrough_save_retry(cx));
        }
        if plan.is_some() {
            if self.walkthrough.rows.is_empty() {
                area = area.child(
                    div()
                        .p_3()
                        .text_xs()
                        .text_color(theme::muted())
                        .child("No review items match this search and the active filters."),
                );
            }
            area = area.child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        list(
                            self.walkthrough.list.clone(),
                            cx.processor(|detail, index, _, cx| {
                                detail.render_walkthrough_row(index, cx)
                            }),
                        )
                        .size_full(),
                    )
                    .child(VerticalScrollbar::new(
                        "walkthrough-scrollbar",
                        ScrollbarTarget::List(self.walkthrough.list.clone()),
                    )),
            );
        }
        area.into_any_element()
    }

    /// Offers retry or discard for a completed guided review whose save failed.
    fn render_walkthrough_save_retry(cx: &mut Context<Self>) -> AnyElement {
        // Keep both recovery actions beside the explanation of what retry preserves.
        div()
            .px_3()
            .pb_2()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme::muted())
                    .child("Retry save uses the guided review already created."),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("retry-guided-review-save", "Retry save").on_click(
                            cx.listener(|detail, _, _, cx| {
                                detail.retry_guided_review_save(cx);
                            }),
                        ),
                    )
                    .child(
                        Button::new("discard-guided-review-save", "Discard")
                            .ghost()
                            .on_click(cx.listener(|detail, _, _, cx| {
                                detail.cancel_smart_filter(cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    /// Each row has bounded content, so a bucket with thousands of files still lays out only the viewport.
    fn render_walkthrough_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.walkthrough.rows.get(index).copied() else {
            return div().into_any_element();
        };
        match row {
            WalkthroughRow::Bucket(bucket_index) => {
                self.render_walkthrough_bucket(bucket_index, cx)
            }
            WalkthroughRow::Actions(bucket_index) => {
                self.render_walkthrough_actions(bucket_index, cx)
            }
            WalkthroughRow::Guidance(bucket) => {
                Self::render_reading_hint(index, &self.walkthrough.buckets[bucket].description)
            }
            WalkthroughRow::Item(id) => self.render_walkthrough_item(id, cx),
            WalkthroughRow::GroupMembers(id) => {
                let Some(item) = self.walkthrough.item(id) else {
                    return div().into_any_element();
                };
                let expanded = self.walkthrough.expanded_items.contains(&id);
                let label = format!(
                    "{} {} members",
                    if expanded { "Hide" } else { "Show" },
                    item.shown
                );
                div().w_full().min_h(px(36.)).px_3().pb_2().bg(theme::panel()).flex().flex_wrap().items_center().gap_1()
                    .child(Button::new(("group-members",index),label).ghost().on_click(cx.listener(move |detail,_,_,cx| detail.toggle_walkthrough_members(id,cx))))
                    .child(div().flex_1())
                    .child(div().id(("group-progress",index)).text_size(px(10.)).text_color(theme::subtle())
                        .tooltip(TextTooltip::builder(format!("All {} files are covered by the checkbox; {} are hidden by filters.",item.files.len(),item.files.len()-item.shown)))
                        .child(NumberFlow::label(format!("{}/{} viewed",item.viewed,item.files.len()))))
                    .into_any_element()
            }
            WalkthroughRow::File { id, file } => div()
                .w_full()
                .pl_4()
                .bg(theme::panel())
                .child(self.render_navigation_file(
                    file,
                    "plan-member",
                    super::navigation_file_target::NavigationFileTarget::GuidedItem(id),
                    cx,
                ))
                .into_any_element(),
        }
    }

    /// Shaded section headers separate buckets from their ordered review stops.
    fn render_walkthrough_bucket(&self, bucket_index: usize, cx: &mut Context<Self>) -> AnyElement {
        let bucket = &self.walkthrough.buckets[bucket_index];
        div()
            .id(("walkthrough-bucket", bucket_index))
            .w_full()
            .min_h(px(66.))
            .py_3()
            .bg(theme::surface())
            .px_3()
            .flex()
            .items_center()
            .gap_1()
            .border_t_1()
            .border_color(theme::border())
            .cursor_pointer()
            .hover(|style| style.bg(theme::hover()))
            .on_click(cx.listener(move |detail, _, _, cx| {
                detail.toggle_walkthrough_bucket(bucket_index, cx);
            }))
            .child(FoldChevron::new(
                ("plan-fold", bucket_index),
                FoldState::of(self.walkthrough.open_bucket != Some(bucket.order)),
            ))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .id(("bucket-title", bucket_index))
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .line_height(px(18.))
                            .tooltip(TextTooltip::builder(bucket.label.clone()))
                            .child(if bucket.order == 0 {
                                bucket.label.clone()
                            } else {
                                format!("{}. {}", bucket.order, bucket.label)
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(theme::subtle())
                            .child(NumberFlow::label(bucket.progress_label())),
                    ),
            )
            .into_any_element()
    }

    /// Draws a bucket's actions row.
    fn render_walkthrough_actions(
        &self,
        bucket_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let bucket = &self.walkthrough.buckets[bucket_index];
        let order = bucket.order;
        div()
            .w_full()
            .min_h(px(36.))
            .bg(theme::surface())
            .pb_1()
            .px_2()
            .flex()
            .items_center()
            .justify_between()
            .child(
                Button::new(
                    ("next-item", bucket_index),
                    if self.side_panel_width() < 300. {
                        "Next"
                    } else {
                        "Next in order"
                    },
                )
                .ghost()
                .disabled(self.walkthrough.next(bucket_index).is_none())
                .on_click(cx.listener(move |detail, _, _, cx| {
                    detail.next_walkthrough_item(bucket_index, cx);
                })),
            )
            .child(
                Button::new(
                    ("bucket-viewed", bucket_index),
                    if self.side_panel_width() < 300. {
                        "Mark all viewed"
                    } else {
                        "Mark bucket viewed"
                    },
                )
                .ghost()
                .disabled(bucket.viewed == bucket.files.len())
                .on_click(cx.listener(move |detail, _, _, cx| {
                    detail.mark_walkthrough_bucket(bucket_index, cx);
                })),
            )
            .child(
                div()
                    .id(("bucket-guidance", bucket_index))
                    .tooltip(TextTooltip::builder(
                        "Show or hide this bucket's review guidance",
                    ))
                    .child(
                        Button::new(("guidance", bucket_index), "?")
                            .ghost()
                            .on_click(cx.listener(move |detail, _, _, cx| {
                                detail.toggle_walkthrough_guidance(order, cx);
                            })),
                    ),
            )
            .into_any_element()
    }

    /// Bucket guidance grows with its copy and stays readable at the narrowest drawer width.
    fn render_reading_hint(index: usize, text: &str) -> AnyElement {
        div()
            .id(("reading-hint", index))
            .w_full()
            .px_3()
            .pt_1()
            .pb_3()
            .bg(theme::surface())
            .flex()
            .flex_col()
            .gap_1()
            .text_size(px(12.))
            .line_height(px(18.))
            .child(
                div()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .child("Review guidance"),
            )
            .child(div().text_color(theme::muted()).child(text.to_owned()))
            .into_any_element()
    }
}

/// Describes the current guided review generation or repair phase.
fn progress_label(progress: Option<AgentProgress>) -> String {
    match progress {
        None => "Starting Claude…".into(),
        Some(AgentProgress::FetchingCode {
            percent: Some(percent),
        }) => format!("Fetching code… {percent}%"),
        Some(AgentProgress::FetchingCode { percent: None } | AgentProgress::UnpackingCode) => {
            "Preparing code…".into()
        }
        Some(AgentProgress::Started | AgentProgress::Thinking) => "Planning review order…".into(),
        Some(AgentProgress::Exploring { tool_calls }) => {
            format!("Reading files… {tool_calls} reads")
        }
        Some(AgentProgress::Writing { .. }) => "Writing buckets…".into(),
        Some(AgentProgress::ReviewingBrief { pages, issues }) => {
            if issues > 0 {
                format!(
                    "Repairing the brief… {}",
                    counted(issues, "issue", "issues")
                )
            } else {
                format!(
                    "Building the brief… {} checked",
                    counted(pages, "page", "pages")
                )
            }
        }
        Some(AgentProgress::ReviewingGuide {
            buckets,
            covered_files,
            total_files,
            issues,
        }) => {
            if issues > 0 {
                format!("Repairing guide… {}", counted(issues, "issue", "issues"))
            } else {
                format!(
                    "Building guide… {} · {covered_files}/{total_files} files",
                    counted(buckets, "bucket", "buckets")
                )
            }
        }
    }
}
