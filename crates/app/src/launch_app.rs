use gpui::{
    App, Application, Bounds, DisplayId, Pixels, Size, TitlebarOptions, WindowBounds,
    WindowOptions, point, prelude::*, px, size,
};

use crate::app_commands::install_app_commands;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::main_window::MainWindow;
use crate::ui_state::{UiStateRecorder, window_bounds_from_placement};

const WINDOW_TITLE: &str = "Quarterdeck";
const WINDOW_WIDTH: f32 = 1200.;
const WINDOW_HEIGHT: f32 = 800.;
const WINDOW_MIN_WIDTH: f32 = 720.;
const WINDOW_MIN_HEIGHT: f32 = 480.;
/// Where the traffic lights sit: centered in the top bar's 40 px.
const TRAFFIC_LIGHTS_X: f32 = 14.;
const TRAFFIC_LIGHTS_Y: f32 = 13.;

/// Initializes the services, starts gpui and opens the main window where it
/// was last placed, on the display it was on. Exits the process when the database cannot be opened.
pub fn launch_app() {
    // Services and UI state first: without the database there is nothing to show.
    let opened = AppServices::initialize().and_then(|services| {
        let recorder = UiStateRecorder::open_default()?;
        Ok((services, recorder))
    });
    let (services, recorder) = match opened {
        Ok(opened) => opened,
        Err(error) => {
            eprintln!("failed to initialize: {error:#}");
            std::process::exit(1);
        }
    };

    Application::new().run(move |cx: &mut App| {
        install_app_commands(cx);
        let min_size = size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT));
        let restored = restore_window(&recorder, min_size, cx);
        cx.set_global(AppSettings::load(&recorder));
        cx.set_global(services.clone());
        cx.set_global(recorder);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(restored.bounds),
                display_id: restored.display_id,
                window_min_size: Some(min_size),
                // The app draws the title bar (the queue's top bar); the traffic lights sit in it.
                titlebar: Some(TitlebarOptions {
                    title: Some(WINDOW_TITLE.into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(TRAFFIC_LIGHTS_X), px(TRAFFIC_LIGHTS_Y))),
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| MainWindow::new(services, window, cx)),
        )
        .expect("failed to open the main window");
        cx.activate(true);
    });
}

/// Where the main window opens.
struct RestoredWindow {
    bounds: WindowBounds,
    /// The display the bounds are relative to; `None` for the main display.
    display_id: Option<DisplayId>,
}

/// The saved placement on the display it was saved on, when that display
/// is connected and the window still fits it. A missing display falls back
/// to the main one with the same placement if it fits there; a placement
/// that fits nowhere opens centered at the default size.
fn restore_window(recorder: &UiStateRecorder, min_size: Size<Pixels>, cx: &App) -> RestoredWindow {
    let placement = recorder.saved_window_placement();

    // The saved display by its stable UUID, or else the main display.
    let saved_display = placement.as_ref().and_then(|placement| {
        let uuid = placement.display.as_ref()?;
        cx.displays().into_iter().find(|display| {
            display
                .uuid()
                .is_ok_and(|candidate| candidate.to_string() == uuid.as_str())
        })
    });
    let display = saved_display.or_else(|| cx.primary_display());
    let display_id = display.as_ref().map(|display| display.id());

    // The saved bounds if they fit that display, else centered on it.
    let bounds = placement
        .zip(display)
        .and_then(|(placement, display)| {
            window_bounds_from_placement(&placement, min_size, display.bounds().size)
        })
        .unwrap_or_else(|| {
            WindowBounds::Windowed(Bounds::centered(
                display_id,
                size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)),
                cx,
            ))
        });
    RestoredWindow { bounds, display_id }
}
