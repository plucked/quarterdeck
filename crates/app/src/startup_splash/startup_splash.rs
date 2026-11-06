use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::channel::mpsc::{self, TryRecvError};
use gpui::{
    Animation, AnimationExt as _, AnyWindowHandle, App, Bounds, Context, DisplayId, MouseButton,
    RenderImage, Task, Window, WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind,
    WindowOptions, div, ease_in_out, img, prelude::*, px, size,
};

use super::decode_splash_frames::decode_splash_frames;
use super::splash_frame::SplashFrame;
use super::splash_playback::{FramePoll, PlaybackStep, SplashPlayback, TimedFrame};
use crate::main_window::MainWindow;
use crate::theme;

/// How long the splash fades out while the main window fades in.
pub const STARTUP_FADE: Duration = Duration::from_secs(1);
/// The splash window's size in points: the movie's 1000×300 pixels.
const SPLASH_WIDTH: f32 = 1000.;
const SPLASH_HEIGHT: f32 = 300.;
const SPLASH_CORNER_RADIUS: f32 = 16.;
/// The running version, shown in the splash's bottom right corner.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// How far the version sits from the splash's right and bottom edges.
const VERSION_INSET_RIGHT: f32 = 20.;
const VERSION_INSET_BOTTOM: f32 = 14.;
/// How long the fade waits for the main window's first frame at most (it
/// draws none while on another space), before it starts anyway.
const MAX_FIRST_DRAW_WAIT: Duration = Duration::from_millis(500);

/// Where the splash is in its life.
enum SplashPhase {
    /// The movie plays; the main window waits hidden.
    Playing,
    /// The movie ended or was clicked away: the main window is shown, still
    /// see-through, and draws its first frame, which takes a moment with a
    /// loaded queue. The splash keeps its picture meanwhile, so the fade
    /// does not start with a jump.
    Revealing,
    /// The splash fades out (on its last frame) while the main window fades
    /// in, then it closes.
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
    /// Frames from the decoding thread; dropping it (with the window) stops
    /// the decoding.
    frames: mpsc::Receiver<SplashFrame>,
    /// Picks the frame to show on each screen refresh.
    playback: SplashPlayback<Arc<RenderImage>>,
    /// The splash window, closed once the fade is over.
    window_handle: AnyWindowHandle,
    /// Starts the fade if the main window's first frame does not come.
    reveal_wait_task: Option<Task<()>>,
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

    /// Starts decoding the movie, and playing it in step with the screen's
    /// refreshes once a few frames are ready.
    fn new(
        video: PathBuf,
        main_window: WindowHandle<MainWindow>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.on_next_frame(window, Self::on_screen_refresh);
        Self {
            frame: None,
            phase: SplashPhase::Playing,
            main_window,
            frames: decode_splash_frames(video),
            playback: SplashPlayback::default(),
            window_handle: window.window_handle(),
            reveal_wait_task: None,
            close_task: None,
        }
    }

    /// Runs before each frame the screen draws (gpui calls it in step with
    /// the display): shows the frame due now, and asks to run again on the
    /// next refresh until the movie is over, which starts the fade (also
    /// when it could not be read).
    fn on_screen_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Take decoded frames without waiting; the decoder runs ahead on its own thread.
        let frames = &mut self.frames;
        let mut poll = || match frames.try_recv() {
            Ok(frame) => FramePoll::Frame(TimedFrame {
                frame: frame.image,
                time: frame.presentation_time,
            }),
            Err(TryRecvError::Empty) => FramePoll::Pending,
            Err(TryRecvError::Closed) => FramePoll::Finished,
        };
        match self.playback.tick(Instant::now(), &mut poll) {
            PlaybackStep::Show(image) => self.show_frame(image, window, cx),
            PlaybackStep::Hold => {}
            PlaybackStep::Ended => {
                self.fade_out(window, cx);
                return;
            }
        }
        cx.on_next_frame(window, Self::on_screen_refresh);
    }

    /// Puts `image` on screen and frees the previous frame's copy on the
    /// GPU (gpui keeps every image it drew until told otherwise).
    fn show_frame(&mut self, image: Arc<RenderImage>, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(previous) = self.frame.replace(image) {
            window.drop_image(previous).ok();
        }
        cx.notify();
    }

    /// Shows the main window, then fades the splash out once the main
    /// window has drawn its first frame (as the main window fades in), and
    /// closes the splash; called when the movie ends and on a click. The
    /// movie keeps playing under the fade when a click cut it short.
    fn fade_out(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !matches!(self.phase, SplashPhase::Playing) {
            return;
        }
        self.phase = SplashPhase::Revealing;
        reveal_main_window(self.main_window, cx);

        // The main window's second refresh comes after its first frame is drawn.
        let splash = cx.entity().downgrade();
        let is_waiting = self
            .main_window
            .update(cx, |_, main_window, _| {
                main_window.on_next_frame(move |main_window, _| {
                    main_window.on_next_frame(move |_, cx| {
                        splash.update(cx, Self::start_fade).ok();
                    });
                });
            })
            .is_ok();
        if !is_waiting {
            self.start_fade(cx);
            return;
        }

        // A main window that draws nothing (on another space) must not hold the splash up.
        self.reveal_wait_task = Some(cx.spawn_in(window, async move |splash, cx| {
            cx.background_executor().timer(MAX_FIRST_DRAW_WAIT).await;
            splash.update(cx, Self::start_fade).ok();
        }));
    }

    /// Fades the splash out over [`STARTUP_FADE`], then closes it; once.
    fn start_fade(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.phase, SplashPhase::Revealing) {
            return;
        }
        self.phase = SplashPhase::FadingOut;
        let window_handle = self.window_handle;
        self.close_task = Some(cx.spawn(async move |_, cx| {
            cx.background_executor().timer(STARTUP_FADE).await;
            window_handle
                .update(cx, |_, window, _| window.remove_window())
                .ok();
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
        // The movie, with the version in its bottom right corner once it shows.
        let picture = div()
            .relative()
            .size_full()
            .children(
                self.frame
                    .clone()
                    .map(|image| img(image).size_full().rounded(px(SPLASH_CORNER_RADIUS))),
            )
            .when(self.frame.is_some(), |picture| {
                picture.child(
                    div()
                        .absolute()
                        .right(px(VERSION_INSET_RIGHT))
                        .bottom(px(VERSION_INSET_BOTTOM))
                        .font_family(theme::UI_FONT)
                        .text_xs()
                        .text_color(theme::on_splash())
                        .child(format!("v{APP_VERSION}")),
                )
            })
            // A click skips the rest of the movie.
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|splash, _, window, cx| splash.fade_out(window, cx)),
            );
        match self.phase {
            SplashPhase::Playing | SplashPhase::Revealing => picture.into_any_element(),
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
