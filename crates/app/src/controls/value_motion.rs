//! Timing and the system motion preference shared by numeric feedback.

use std::time::{Duration, Instant};

pub(super) const DURATION: Duration = Duration::from_millis(450);

pub(super) fn eased_progress(start: Instant, now: Instant) -> f32 {
    let elapsed =
        (now.saturating_duration_since(start).as_secs_f32() / DURATION.as_secs_f32()).min(1.);
    1. - (1. - elapsed).powi(5)
}

#[cfg(target_os = "macos")]
pub(super) fn reduce_motion() -> bool {
    objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
}

#[cfg(not(target_os = "macos"))]
pub(super) fn reduce_motion() -> bool {
    false
}
