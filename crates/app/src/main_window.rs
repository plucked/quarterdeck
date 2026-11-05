use gpui::{
    Animation, AnimationExt as _, Context, Entity, Subscription, Task, Window,
    WindowBackgroundAppearance, div, ease_in_out, prelude::*,
};

use qd_cache::DisplayUuid;

use crate::app_services::AppServices;
use crate::review_queue::ReviewQueue;
use crate::startup_splash::STARTUP_FADE;
use crate::theme;
use crate::ui_state::{UiStateRecorder, placement_from_window};

/// Whether the main window shows yet.
#[derive(Clone, Copy)]
pub enum MainWindowReveal {
    /// Opened hidden and see-through while the startup splash plays.
    AfterSplash,
    /// Fading in as the splash fades out.
    FadingIn,
    /// Shown, with its opaque background.
    Shown,
}

/// The root view of the main window: owns the review queue and records the
/// window's placement as UI state.
pub struct MainWindow {
    review_queue: Entity<ReviewQueue>,
    reveal: MainWindowReveal,
    /// Makes the background opaque again once the fade-in is over.
    reveal_task: Option<Task<()>>,
    /// Keep the bounds and activation observers registered for the window's lifetime.
    _subscriptions: [Subscription; 2],
}

impl MainWindow {
    pub fn new(
        services: AppServices,
        reveal: MainWindowReveal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
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
            reveal,
            reveal_task: None,
            _subscriptions: subscriptions,
        }
    }

    /// Shows the window, opened hidden behind the startup splash, and fades
    /// its content in; once faded in, its background turns opaque again.
    pub fn reveal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.reveal, MainWindowReveal::AfterSplash) {
            return;
        }
        self.reveal = MainWindowReveal::FadingIn;
        window.activate_window();
        self.reveal_task = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(STARTUP_FADE).await;
            this.update_in(cx, |main_window, window, cx| {
                main_window.reveal = MainWindowReveal::Shown;
                window.set_background_appearance(WindowBackgroundAppearance::Opaque);
                cx.notify();
            })
            .ok();
        }));
        cx.notify();
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
        let content = div()
            .size_full()
            .font_family(theme::UI_FONT)
            .child(self.review_queue.clone());
        match self.reveal {
            // Fully transparent until revealed, so nothing flashes as the window appears.
            MainWindowReveal::AfterSplash => content.opacity(0.).into_any_element(),
            MainWindowReveal::FadingIn => content
                .with_animation(
                    "main-window-fade-in",
                    Animation::new(STARTUP_FADE).with_easing(ease_in_out),
                    Styled::opacity,
                )
                .into_any_element(),
            MainWindowReveal::Shown => content.into_any_element(),
        }
    }
}
