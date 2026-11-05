use gpui::{
    App, Application, Bounds, Pixels, Size, TitlebarOptions, WindowBounds, WindowOptions,
    prelude::*, px, size,
};

use crate::app_commands::install_app_commands;
use crate::app_services::AppServices;
use crate::main_window::MainWindow;
use crate::ui_state::{UiStateRecorder, window_bounds_from_placement};

const WINDOW_TITLE: &str = "Quick Check";
const WINDOW_WIDTH: f32 = 1200.;
const WINDOW_HEIGHT: f32 = 800.;
const WINDOW_MIN_WIDTH: f32 = 720.;
const WINDOW_MIN_HEIGHT: f32 = 480.;

/// Initializes the services, starts gpui and opens the main window where it
/// was last placed. Exits the process when the database cannot be opened.
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
        let window_bounds = restored_window_bounds(&recorder, min_size, cx);
        cx.set_global(recorder);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                window_min_size: Some(min_size),
                titlebar: Some(TitlebarOptions {
                    title: Some(WINDOW_TITLE.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| MainWindow::new(services, window, cx)),
        )
        .expect("failed to open the main window");
        cx.activate(true);
    });
}

/// The saved window placement when it still fits a connected display,
/// otherwise a centered window of the default size.
fn restored_window_bounds(
    recorder: &UiStateRecorder,
    min_size: Size<Pixels>,
    cx: &App,
) -> WindowBounds {
    let displays: Vec<Bounds<Pixels>> = cx
        .displays()
        .iter()
        .map(|display| display.bounds())
        .collect();
    recorder
        .saved_window_placement()
        .and_then(|placement| window_bounds_from_placement(&placement, min_size, &displays))
        .unwrap_or_else(|| {
            WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)),
                cx,
            ))
        })
}
