use gpui::{
    App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size,
};

use crate::app_commands::install_app_commands;
use crate::app_services::AppServices;
use crate::review_queue::ReviewQueue;

const WINDOW_TITLE: &str = "Quick Check";
const WINDOW_WIDTH: f32 = 1200.;
const WINDOW_HEIGHT: f32 = 800.;
const WINDOW_MIN_WIDTH: f32 = 720.;
const WINDOW_MIN_HEIGHT: f32 = 480.;

/// Initializes the services, starts gpui and opens the review queue window.
/// Exits the process when the cache cannot be opened.
pub fn launch_app() {
    // Services first: without a cache there is nothing to show.
    let services = match AppServices::initialize() {
        Ok(services) => services,
        Err(error) => {
            eprintln!("failed to initialize: {error:#}");
            std::process::exit(1);
        }
    };

    Application::new().run(move |cx: &mut App| {
        install_app_commands(cx);

        // One centered main window holding the review queue.
        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT))),
                titlebar: Some(TitlebarOptions {
                    title: Some(WINDOW_TITLE.into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| ReviewQueue::new(services, window, cx)),
        )
        .expect("failed to open the main window");
        cx.activate(true);
    });
}
