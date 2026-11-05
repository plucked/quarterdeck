use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::StreamExt as _;
use gpui::{
    Animation, AnimationExt as _, App, Bounds, Context, DisplayId, MouseButton, RenderImage, Task,
    Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, div,
    ease_in_out, img, prelude::*, px, size,
};

use super::decode_splash_frames::decode_splash_frames;
use crate::main_window::MainWindow;

/// How long the splash fades out while the main window fades in.
pub const STARTUP_FADE: Duration = Duration::from_secs(1);
/// The splash window's size in points: the movie's 1000×300 pixels.
const SPLASH_WIDTH: f32 = 1000.;
const SPLASH_HEIGHT: f32 = 300.;
const SPLASH_CORNER_RADIUS: f32 = 16.;

/// Where the splash is in its life.
enum SplashPhase {
    /// The movie plays; the main window waits hidden.
    Playing,
    /// The movie ended or was clicked away: the splash fades out (on its
    /// last frame) while the main window fades in, then it closes.
    FadingOut,
}

/// The startup splash: a borderless, see-through window playing the
/// startup movie centered on the screen the main window opens on. The main
/// window is already open, hidden, so its cached queue shows and its GitHub
/// refresh runs during the movie.
pub struct StartupSplash {
    /// The frame on screen; `None` until the first one is decoded, so the
    /// window shows nothing while the movie starts.
    frame: Option<Arc<RenderImage>>,
    phase: SplashPhase,
    main_window: WindowHandle<MainWindow>,
    /// Shows each frame on time, then starts the fade. Dropping it (with the
    /// window) stops the decoding.
    _playback_task: Task<()>,
    /// Closes the window once the fade is over.
    close_task: Option<Task<()>>,
}

impl StartupSplash {
    /// Opens the splash on `display_id` (the main display for `None`),
    /// playing the movie at `video`; `main_window`, opened hidden, is
    /// revealed when it ends. Without a splash window the main window shows
    /// at once.
    pub fn open(
        video: PathBuf,
        main_window: WindowHandle<MainWindow>,
        display_id: Option<DisplayId>,
        cx: &mut App,
    ) {
        let bounds = Bounds::centered(display_id, size(px(SPLASH_WIDTH), px(SPLASH_HEIGHT)), cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            display_id,
            // No title bar and a clear background: only the movie's rounded picture shows.
            titlebar: None,
            window_background: WindowBackgroundAppearance::Transparent,
            kind: WindowKind::PopUp,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            ..Default::default()
        };
        let opened = cx.open_window(options, |window, cx| {
            cx.new(|cx| Self::new(video, main_window, window, cx))
        });
        if let Err(error) = opened {
            eprintln!("cannot open the startup splash: {error:#}");
            reveal_main_window(main_window, cx);
        }
    }

    /// Starts playing the movie as it decodes.
    fn new(
        video: PathBuf,
        main_window: WindowHandle<MainWindow>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let playback_task = cx.spawn_in(window, async move |this, cx| {
            let mut frames = decode_splash_frames(video);
            // The clock starts with the first decoded frame: when it showed, and its movie time.
            let mut clock: Option<(Instant, Duration)> = None;
            while let Some(frame) = frames.next().await {
                // Wait until the frame is due. A late frame shows at once, and the
                // frames after it catch up within one redraw.
                let (started, first_time) =
                    *clock.get_or_insert((Instant::now(), frame.presentation_time));
                let due = started + frame.presentation_time.saturating_sub(first_time);
                let wait = due.saturating_duration_since(Instant::now());
                if !wait.is_zero() {
                    cx.background_executor().timer(wait).await;
                }
                let shown = this.update_in(cx, |splash, window, cx| {
                    splash.show_frame(frame.image, window, cx);
                });
                if shown.is_err() {
                    return;
                }
            }
            // The movie ended, or could not be read: on to the app either way.
            this.update_in(cx, Self::fade_out).ok();
        });
        Self {
            frame: None,
            phase: SplashPhase::Playing,
            main_window,
            _playback_task: playback_task,
            close_task: None,
        }
    }

    /// Puts `image` on screen and frees the previous frame's copy on the
    /// GPU (gpui keeps every image it drew until told otherwise).
    fn show_frame(&mut self, image: Arc<RenderImage>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(previous) = self.frame.replace(image) {
            window.drop_image(previous).ok();
        }
        cx.notify();
    }

    /// Fades the splash out and the main window in, then closes the splash;
    /// called when the movie ends and on a click. The movie keeps playing
    /// under the fade when a click cut it short.
    fn fade_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.phase, SplashPhase::FadingOut) {
            return;
        }
        self.phase = SplashPhase::FadingOut;
        reveal_main_window(self.main_window, cx);
        self.close_task = Some(cx.spawn_in(window, async move |_, cx| {
            cx.background_executor().timer(STARTUP_FADE).await;
            cx.update(|window, _| window.remove_window()).ok();
        }));
        cx.notify();
    }
}

/// Shows the main window and starts its fade-in; nothing when it was closed.
fn reveal_main_window(main_window: WindowHandle<MainWindow>, cx: &mut App) {
    main_window.update(cx, MainWindow::reveal).ok();
}

impl Render for StartupSplash {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let picture = div()
            .size_full()
            .children(
                self.frame
                    .clone()
                    .map(|image| img(image).size_full().rounded(px(SPLASH_CORNER_RADIUS))),
            )
            // A click skips the rest of the movie.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|splash, _, window, cx| splash.fade_out(window, cx)),
            );
        match self.phase {
            SplashPhase::Playing => picture.into_any_element(),
            SplashPhase::FadingOut => picture
                .with_animation(
                    "startup-splash-fade-out",
                    Animation::new(STARTUP_FADE).with_easing(ease_in_out),
                    |picture, delta| picture.opacity(1. - delta),
                )
                .into_any_element(),
        }
    }
}
