//! A progress fill that follows changes without affecting surrounding layout.

use std::time::Instant;

use gpui::{
    App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Rgba, Style, Window, fill, px, relative, size,
};

use super::value_motion::{DURATION, eased_progress, reduce_motion};

const HEIGHT: f32 = 4.;

pub struct ProgressBar {
    id: ElementId,
    fraction: f32,
    track: Rgba,
    fill: Rgba,
}

impl ProgressBar {
    pub fn new(id: impl Into<ElementId>, fraction: f32, track: Rgba, fill: Rgba) -> Self {
        Self {
            id: id.into(),
            fraction: if fraction.is_finite() {
                fraction.clamp(0., 1.)
            } else {
                0.
            },
            track,
            fill,
        }
    }
}

impl IntoElement for ProgressBar {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for ProgressBar {
    type RequestLayoutState = ();
    type PrepaintState = (f32, bool);

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let style = Style {
            size: size(relative(1.).into(), px(HEIGHT).into()),
            flex_shrink: 0.,
            ..Style::default()
        };
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        let now = Instant::now();
        let animated = !reduce_motion();
        window.with_element_state(
            id.expect("ProgressBar has an id"),
            |state: Option<Progress>, _| {
                let mut state = state.unwrap_or_else(|| Progress::new(self.fraction));
                state.update(self.fraction, now, animated);
                ((state.value(now), state.is_active(now)), state)
            },
        )
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        (): &mut (),
        (fraction, active): &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let radius = bounds.size.height / 2.;
        window.paint_quad(fill(bounds, self.track).corner_radii(radius));
        if *fraction > 0. {
            let filled = Bounds::new(
                bounds.origin,
                size(bounds.size.width * *fraction, bounds.size.height),
            );
            window.paint_quad(
                fill(filled, self.fill).corner_radii(radius.min(filled.size.width / 2.)),
            );
        }
        // Paint runs only while visible. Settled bars request no frames.
        if *active {
            window.request_animation_frame();
        }
    }
}

struct Progress {
    from: f32,
    target: f32,
    started: Option<Instant>,
}

impl Progress {
    fn new(value: f32) -> Self {
        Self {
            from: value,
            target: value,
            started: None,
        }
    }

    fn update(&mut self, target: f32, now: Instant, animated: bool) {
        if !animated {
            *self = Self::new(target);
        } else if target.to_bits() != self.target.to_bits() {
            // Retarget from the visible fill, including a mid-flight reversal.
            self.from = self.value(now);
            self.target = target;
            self.started = Some(now);
        } else if !self.is_active(now) {
            self.started = None;
        }
    }

    fn value(&self, now: Instant) -> f32 {
        self.started.map_or(self.target, |start| {
            self.from + (self.target - self.from) * eased_progress(start, now)
        })
    }

    fn is_active(&self, now: Instant) -> bool {
        self.started
            .is_some_and(|start| now.saturating_duration_since(start) < DURATION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_and_unchanged_values_do_not_animate() {
        let now = Instant::now();
        let mut progress = Progress::new(0.4);
        progress.update(0.4, now, true);
        assert!(!progress.is_active(now));
        assert!((progress.value(now) - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn fills_and_empties_without_restarting_on_rerender() {
        let now = Instant::now();
        let mut progress = Progress::new(0.);
        progress.update(1., now, true);
        let mid = now + DURATION / 2;
        assert!(progress.value(mid) > 0. && progress.value(mid) < 1.);
        progress.update(1., mid, true);
        assert!(!progress.is_active(now + DURATION));
        assert!((progress.value(now + DURATION) - 1.).abs() < f32::EPSILON);
        progress.update(0., now + DURATION, true);
        assert!(progress.value(now + DURATION + DURATION / 2) > 0.);
        assert!(progress.value(now + DURATION * 2).abs() < f32::EPSILON);
    }

    #[test]
    fn rapid_reversals_continue_from_the_visible_fill() {
        let now = Instant::now();
        let mut progress = Progress::new(0.2);
        progress.update(1., now, true);
        let interrupted = now + DURATION / 3;
        let visible = progress.value(interrupted);
        progress.update(0., interrupted, true);
        assert!((progress.value(interrupted) - visible).abs() < f32::EPSILON);
        assert!(progress.value(interrupted + DURATION / 2) < visible);
        assert!(!progress.is_active(interrupted + DURATION));
    }

    #[test]
    fn reduced_motion_settles_an_active_transition() {
        let now = Instant::now();
        let mut progress = Progress::new(0.);
        progress.update(1., now, true);
        progress.update(1., now + DURATION / 3, false);
        assert!(!progress.is_active(now + DURATION / 3));
        assert!((progress.value(now + DURATION / 3) - 1.).abs() < f32::EPSILON);
        progress.update(1., now + DURATION / 2, true);
        assert!(!progress.is_active(now + DURATION / 2));
    }
}
