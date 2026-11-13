use gpui::{AnyElement, ClickEvent, Context, IntoElement, div, prelude::*, px};
use qd_file_tree::FacetCount;
use qd_studio::{AgentProgress, SmartBucket};

use super::pull_request_detail::PullRequestDetail;
use crate::controls::{Button, CheckState, Checkbox, NumberFlow, PulsingStatus, TextTooltip};
use crate::display_text::{short_sha, time_ago, unix_now};
use crate::facet_chips::{ChipClick, FacetChip, render_chip_row};
use crate::theme;

impl PullRequestDetail {
    pub(super) fn render_smart_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.current_smart_filter();
        let running = state.is_some_and(|state| state.run.is_some());
        let filter = state.and_then(|state| state.filter.as_ref());
        let is_outdated = filter.is_some_and(|filter| self.smart_filter_is_stale(filter));
        let label = if filter.is_some() {
            "Regenerate"
        } else {
            "Generate smart filter"
        };
        let mut content = div().w_full().min_w_0().flex().flex_col().gap_1p5();
        if let Some(filter) = filter {
            content = content.child(
                div()
                    .id("smart-buckets")
                    .max_h(px(192.))
                    .overflow_y_scroll()
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .content_start()
                    .gap_1()
                    .children(
                        filter
                            .buckets
                            .iter()
                            .map(|bucket| self.render_smart_bucket(bucket, is_outdated, cx)),
                    ),
            );
        }
        content = content.child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .child(
                    Button::new("generate-smart-filter", label)
                        .ghost()
                        .disabled(running || self.shown_head_sha().is_none())
                        .on_click(cx.listener(|detail, _, _, cx| detail.generate_smart_filter(cx))),
                )
                .when(running, |row| {
                    row.child(PulsingStatus::new(
                        "smart-filter-progress",
                        progress_label(state.and_then(|state| state.progress)),
                    ))
                    .child(
                        Button::new("cancel-smart-filter", "Cancel")
                            .ghost()
                            .on_click(
                                cx.listener(|detail, _, _, cx| detail.cancel_smart_filter(cx)),
                            ),
                    )
                }),
        );
        if let Some(filter) = filter {
            let version = if is_outdated {
                "Diff changed since generation"
            } else {
                "Review in numbered order"
            };
            content = content.child(
                div()
                    .id("smart-filter-version")
                    .text_xs()
                    .text_color(if is_outdated {
                        theme::removed()
                    } else {
                        theme::subtle()
                    })
                    .tooltip(TextTooltip::builder(format!(
                        "Generated {} from head {}{}. Review buckets in numbered order.{}",
                        time_ago(i64::try_from(filter.generated_at).unwrap_or(0), unix_now()),
                        short_sha(&filter.head_sha),
                        filter
                            .base_sha
                            .as_ref()
                            .map(|base| format!(", base {base}"))
                            .unwrap_or_default(),
                        if is_outdated {
                            " Regenerate to update the buckets."
                        } else {
                            ""
                        }
                    )))
                    .child(NumberFlow::label(version)),
            );
        }
        if let Some(error) = state.and_then(|state| state.error.as_ref()) {
            content = content.child(
                div()
                    .text_xs()
                    .text_color(theme::danger())
                    .child(error.clone()),
            );
        }
        render_chip_row("Smart", vec![content.into_any_element()])
    }

    fn render_smart_bucket(
        &self,
        bucket: &SmartBucket,
        is_outdated: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let order = bucket.order;
        let count = self
            .file_tree()
            .smart_counts
            .iter()
            .find(|count| count.value == order)
            .cloned()
            .unwrap_or(FacetCount {
                value: order,
                total: 0,
                matching: 0,
            });
        let label = format!("{}. {}", order, bucket.label);
        let hint = if bucket.suggest_viewed {
            "\nSuggested for marking viewed. Use the checkbox to mark this bucket."
        } else {
            ""
        };
        let chip = FacetChip::new(
            format!("smart-bucket-{order}"),
            label.clone(),
            &count,
            self.file_tree().smart_choices.choice_of(&order),
        )
        .tooltip(format!("{label}\n{}{hint}", bucket.description))
        .on_click(cx.listener(move |detail, event: &ClickEvent, _, cx| {
            detail.click_smart_bucket(order, ChipClick::of(event), cx);
        }));
        let mut group = div()
            .min_w_0()
            .max_w_full()
            .flex()
            .items_center()
            .gap_1()
            .child(chip);
        if bucket.suggest_viewed {
            let viewed = bucket
                .files
                .iter()
                .filter(|file| self.file_review().is_viewed(file))
                .count();
            let mark = if viewed == 0 {
                CheckState::Unchecked
            } else if viewed == bucket.files.len() {
                CheckState::Checked
            } else {
                CheckState::Mixed
            };
            let title = if is_outdated {
                "Regenerate for the current diff before marking this bucket viewed"
            } else if mark == CheckState::Checked {
                "Mark bucket not viewed"
            } else {
                "Mark bucket viewed"
            };
            group = group.child(
                div()
                    .id(("smart-viewed", order))
                    .flex_none()
                    .p_1()
                    .tooltip(TextTooltip::builder(title))
                    .when(is_outdated, |mark| mark.opacity(0.4))
                    .when(!is_outdated, |mark| {
                        mark.cursor_pointer()
                            .on_click(cx.listener(move |detail, _, _, cx| {
                                detail.toggle_smart_bucket_viewed(order, cx);
                            }))
                    })
                    .child(Checkbox::new(mark)),
            );
        }
        group.into_any_element()
    }
}

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
    }
}
