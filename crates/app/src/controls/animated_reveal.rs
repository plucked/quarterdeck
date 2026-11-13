//! A clipped row that expands and fades without restarting on ordinary renders.

use std::time::{Duration, Instant};

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window, div, prelude::*,
};

use super::value_motion::reduce_motion;

const OPEN_DURATION: Duration = Duration::from_millis(240);
const CLOSE_DURATION: Duration = Duration::from_millis(160);

/// Reveals `content` within a row of a known expanded height. Keep this
/// element mounted while closed so it can animate in both directions.
pub struct AnimatedReveal {
    id: ElementId,
    expanded: bool,
    height: Pixels,
    content: Option<AnyElement>,
}

impl AnimatedReveal {
    pub fn new(
        id: impl Into<ElementId>,
        expanded: bool,
        height: Pixels,
        content: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            expanded,
            height,
            content: Some(content.into_any_element()),
        }
    }
}

impl IntoElement for AnimatedReveal {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for AnimatedReveal {
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, AnyElement) {
        let now = Instant::now();
        let (amount, active) = window.with_element_state(
            id.expect("AnimatedReveal has an id"),
            |state: Option<RevealMotion>, _| {
                let mut state = state.unwrap_or_else(|| RevealMotion::new(self.expanded));
                state.update(self.expanded, now, !reduce_motion());
                ((state.value(now), state.is_active(now)), state)
            },
        );
        if active {
            window.request_animation_frame();
        }
        let mut row = div()
            .flex_none()
            .h(self.height * amount)
            .overflow_hidden()
            .when(amount > 0., |row| {
                row.child(
                    div()
                        .h(self.height)
                        .opacity(amount)
                        .children(self.content.take()),
                )
            })
            .into_any_element();
        (row.request_layout(window, cx), row)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        row: &mut AnyElement,
        window: &mut Window,
        cx: &mut App,
    ) {
        row.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        row: &mut AnyElement,
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        row.paint(window, cx);
    }
}

struct RevealMotion {
    from: f32,
    expanded: bool,
    started: Option<Instant>,
}

impl RevealMotion {
    fn new(expanded: bool) -> Self {
        Self {
            from: f32::from(expanded),
            expanded,
            started: None,
        }
    }

    fn duration(&self) -> Duration {
        if self.expanded {
            OPEN_DURATION
        } else {
            CLOSE_DURATION
        }
    }

    fn update(&mut self, expanded: bool, now: Instant, animated: bool) {
        if !animated {
            *self = Self::new(expanded);
        } else if expanded != self.expanded {
            self.from = self.value(now);
            self.expanded = expanded;
            self.started = Some(now);
        } else if !self.is_active(now) {
            self.started = None;
        }
    }

    fn value(&self, now: Instant) -> f32 {
        let target = f32::from(self.expanded);
        self.started.map_or(target, |start| {
            let elapsed = (now.saturating_duration_since(start).as_secs_f32()
                / self.duration().as_secs_f32())
            .min(1.);
            let eased = 1. - (1. - elapsed).powi(5);
            self.from + (target - self.from) * eased
        })
    }

    fn is_active(&self, now: Instant) -> bool {
        self.started
            .is_some_and(|start| now.saturating_duration_since(start) < self.duration())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_and_unrelated_renders_do_not_restart_motion() {
        let now = Instant::now();
        let mut motion = RevealMotion::new(false);
        assert!(!motion.is_active(now));
        motion.update(true, now, true);
        motion.update(true, now + OPEN_DURATION / 2, true);
        assert!(!motion.is_active(now + OPEN_DURATION));
        assert!((motion.value(now + OPEN_DURATION) - 1.).abs() < f32::EPSILON);
    }

    #[test]
    fn rapid_tab_changes_reverse_from_the_visible_height() {
        let now = Instant::now();
        let mut motion = RevealMotion::new(false);
        motion.update(true, now, true);
        let interrupted = now + OPEN_DURATION / 4;
        let visible = motion.value(interrupted);
        assert!(visible > 0. && visible < 1.);
        motion.update(false, interrupted, true);
        assert!((motion.value(interrupted) - visible).abs() < f32::EPSILON);
        let reopened = interrupted + CLOSE_DURATION / 4;
        let closing = motion.value(reopened);
        assert!(closing > 0. && closing < visible);
        motion.update(true, reopened, true);
        assert!((motion.value(reopened) - closing).abs() < f32::EPSILON);
        assert!(!motion.is_active(reopened + OPEN_DURATION));
    }

    #[test]
    fn closing_finishes_and_reduced_motion_snaps_to_the_current_choice() {
        let now = Instant::now();
        let mut motion = RevealMotion::new(true);
        motion.update(false, now, true);
        assert!(motion.value(now + CLOSE_DURATION / 2) > 0.);
        assert!(motion.value(now + CLOSE_DURATION).abs() < f32::EPSILON);
        assert!(!motion.is_active(now + CLOSE_DURATION));
        motion.update(true, now + CLOSE_DURATION, true);
        motion.update(true, now + CLOSE_DURATION + OPEN_DURATION / 2, false);
        assert!(!motion.is_active(now + CLOSE_DURATION + OPEN_DURATION / 2));
        assert!((motion.value(now + CLOSE_DURATION + OPEN_DURATION / 2) - 1.).abs() < f32::EPSILON);
    }
}
