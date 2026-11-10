use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, FontWeight, IntoElement, SharedString, Styled, div,
    prelude::*, px, relative,
};

use super::batch_progress::BatchProgress;
use super::pull_request_detail::PullRequestDetail;
use crate::controls::dot_pulse;
use crate::count_as_f32::count_as_f32;
use crate::theme;

/// One sweep of the bar when the share done is unknown.
const SWEEP_PERIOD: Duration = Duration::from_millis(1400);
/// The share of the track the sweeping bar covers.
const SWEEP_WIDTH: f32 = 0.3;
/// The line under the header while something loads.
const PROGRESS_LINE_HEIGHT: f32 = 2.;
/// The bar of the loading card.
const LOADING_BAR_HEIGHT: f32 = 4.;
const LOADING_CARD_WIDTH: f32 = 420.;

/// What the pane is busy with, for the header pill.
struct Activity {
    label: SharedString,
    /// The share done, when known.
    fraction: Option<f32>,
    /// Whether it waits on GitHub (the pill says so with its color).
    is_network: bool,
}

impl PullRequestDetail {
    /// What the pane is busy with now: loading files, checking the stack,
    /// or coloring syntax; `None` when idle.
    fn activity(&self) -> Option<Activity> {
        if let Some(progress) = self.load_progress() {
            return Some(Activity {
                label: progress.label(),
                fraction: progress.fraction(),
                is_network: progress.is_network(),
            });
        }
        if self.is_stack_loading() {
            return Some(Activity {
                label: "Checking the stack with GitHub…".into(),
                fraction: None,
                is_network: true,
            });
        }
        let BatchProgress { done, total } = self.highlight_progress()?;
        Some(Activity {
            label: format!("Coloring syntax… {done} of {total}").into(),
            fraction: Some(share(done, total)),
            is_network: false,
        })
    }

    /// The header's activity pill: a pulsing dot and what is happening.
    pub(super) fn render_activity_pill(&self) -> Option<AnyElement> {
        let activity = self.activity()?;
        let color = if activity.is_network {
            theme::accent()
        } else {
            theme::muted()
        };
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap_1p5()
                .px_2()
                .py_0p5()
                .rounded_full()
                .border_1()
                .border_color(theme::border())
                .bg(theme::surface())
                .text_xs()
                .text_color(theme::muted())
                .child(div().size_2().rounded_full().bg(color).with_animation(
                    "activity-pulse",
                    dot_pulse(),
                    Styled::opacity,
                ))
                .child(activity.label)
                .into_any_element(),
        )
    }

    /// The thin line along the header's bottom edge while something
    /// loads: filled to the share done, or sweeping when that is unknown.
    pub(super) fn render_progress_line(&self) -> Option<AnyElement> {
        let activity = self.activity()?;
        Some(render_bar(
            "header-progress",
            activity.fraction,
            PROGRESS_LINE_HEIGHT,
        ))
    }

    /// The card in place of the files while they load: the step, its bar,
    /// and why a first load from GitHub takes a moment.
    pub(super) fn render_loading_card(&self) -> AnyElement {
        let Some(progress) = self.load_progress() else {
            return div().into_any_element();
        };
        let hint = if progress.is_network() {
            "Large pull requests take a moment the first time; afterwards they open from the cache."
        } else {
            "Almost there."
        };
        div()
            .pb_3()
            .child(
                div()
                    .w(px(LOADING_CARD_WIDTH))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_5()
                    .rounded_lg()
                    .border_1()
                    .border_color(theme::border())
                    .bg(theme::panel())
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child(progress.label()),
                    )
                    .child(render_bar(
                        "loading-card-progress",
                        progress.fraction(),
                        LOADING_BAR_HEIGHT,
                    ))
                    .child(div().text_xs().text_color(theme::subtle()).child(hint)),
            )
            .into_any_element()
    }
}

/// A progress track of `height`: filled to `fraction`, or with a sweeping
/// segment when the share done is unknown.
fn render_bar(id: &'static str, fraction: Option<f32>, height: f32) -> AnyElement {
    let track = div()
        .relative()
        .w_full()
        .h(px(height))
        .rounded_sm()
        .overflow_hidden()
        .bg(theme::border());
    match fraction {
        Some(fraction) => track
            .child(
                div()
                    .h_full()
                    .w(relative(fraction.clamp(0., 1.)))
                    .bg(theme::accent()),
            )
            .into_any_element(),
        None => track
            .child(
                div()
                    .absolute()
                    .top_0()
                    .h_full()
                    .w(relative(SWEEP_WIDTH))
                    .bg(theme::accent())
                    .with_animation(
                        id,
                        Animation::new(SWEEP_PERIOD).repeat(),
                        |segment, delta| {
                            segment.left(relative(delta * (1. + SWEEP_WIDTH) - SWEEP_WIDTH))
                        },
                    ),
            )
            .into_any_element(),
    }
}

/// `done / total` as a fraction for a small count of batches.
fn share(done: usize, total: usize) -> f32 {
    count_as_f32(done) / count_as_f32(total).max(1.)
}
