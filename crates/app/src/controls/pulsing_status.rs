//! `PulsingStatus`: a short accent-colored status that pulses while it
//! lasts, and the shared pulse of the app's activity dots.

use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, App, ElementId, IntoElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::*, pulsating_between,
};

use super::NumberFlow;
use crate::theme;

/// One breath of every pulse: a status text or an activity dot.
const PULSE_PERIOD: Duration = Duration::from_millis(1200);
/// How far a status text fades at the low point of its pulse.
const TEXT_LOWEST_OPACITY: f32 = 0.4;
/// How far an activity dot fades at the low point of its pulse; deeper
/// than text, since a dot carries no words to keep readable.
const DOT_LOWEST_OPACITY: f32 = 0.3;

/// The repeating opacity pulse of an activity dot (the detail's activity
/// pill, a running check's mark), applied with
/// `.with_animation(id, dot_pulse(), Styled::opacity)`.
pub fn dot_pulse() -> Animation {
    pulse_between(DOT_LOWEST_OPACITY)
}

/// A repeating pulse from `lowest_opacity` up to full and back.
fn pulse_between(lowest_opacity: f32) -> Animation {
    Animation::new(PULSE_PERIOD)
        .repeat()
        .with_easing(pulsating_between(lowest_opacity, 1.))
}

/// A small accent-colored status line that pulses while it lasts, such as
/// the top bar's `Refreshing from GitHub…` or the Checks tab's `Reading…`.
#[derive(IntoElement)]
pub struct PulsingStatus {
    id: ElementId,
    text: SharedString,
}

impl PulsingStatus {
    /// A pulsing `text`; `id` keeps its animation apart from its siblings'.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

impl RenderOnce for PulsingStatus {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .text_xs()
            .text_color(theme::accent())
            .child(NumberFlow::new("status-label", self.text))
            .with_animation(self.id, pulse_between(TEXT_LOWEST_OPACITY), Styled::opacity)
    }
}
