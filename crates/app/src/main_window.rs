use gpui::{Context, Entity, Subscription, Window, div, prelude::*};

use qd_cache::DisplayUuid;

use crate::app_services::AppServices;
use crate::review_queue::ReviewQueue;
use crate::theme;
use crate::ui_state::{UiStateRecorder, placement_from_window};

/// The root view of the main window: owns the review queue and records the
/// window's placement as UI state.
pub struct MainWindow {
    review_queue: Entity<ReviewQueue>,
    /// Keep the bounds and activation observers registered for the window's lifetime.
    _subscriptions: [Subscription; 2],
}

impl MainWindow {
    pub fn new(services: AppServices, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let review_queue = cx.new(|cx| ReviewQueue::new(services, window, cx));
        let subscriptions = [
            // Record every move, resize or mode change of the window.
            cx.observe_window_bounds(window, |_, window, cx| {
                let display = window
                    .display(cx)
                    .and_then(|display| display.uuid().ok())
                    .map(|uuid| DisplayUuid::new(uuid.to_string()));
                UiStateRecorder::record_window_placement(
                    cx,
                    placement_from_window(window.window_bounds(), window.viewport_size(), display),
                );
            }),
            cx.observe_window_activation(window, Self::on_window_activation_changed),
        ];
        Self {
            review_queue,
            _subscriptions: subscriptions,
        }
    }

    /// Writes the UI state at once when the window loses focus: switching to
    /// the editor comes before the rebuild that kills and relaunches the app.
    fn on_window_activation_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.is_window_active() {
            return;
        }
        self.review_queue
            .update(cx, |review_queue, cx| review_queue.record_view_state(cx));
        UiStateRecorder::write_soon(cx);
    }
}

impl Render for MainWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .font_family(theme::UI_FONT)
            .child(self.review_queue.clone())
    }
}
