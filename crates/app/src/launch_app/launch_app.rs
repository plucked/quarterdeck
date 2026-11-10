use gpui::{App, Application, TitlebarOptions, WindowOptions, point, prelude::*, px, size};
use qd_studio::{clean_orphan_workspaces, workspace_root};

use super::restore_window::restore_window;
use super::show_startup_failure::show_startup_failure;
use super::startup_mode::StartupMode;

use crate::app_commands::install_app_commands;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::app_update::AppUpdater;
use crate::avatar_images::AvatarImages;
use crate::brief_gallery::open_brief_gallery;
use crate::bundled_fonts::load_bundled_fonts;
use crate::bundled_icons::BundledIcons;
use crate::main_window::MainWindow;
use crate::startup_splash::StartupSplash;
use crate::ui_state::UiStateRecorder;

const WINDOW_TITLE: &str = "Quarterdeck";
const WINDOW_MIN_WIDTH: f32 = 720.;
const WINDOW_MIN_HEIGHT: f32 = 480.;
/// Set (to anything) at launch, the brief gallery opens too; `bacon
/// gallery` sets it, so the gallery comes back after every rebuild.
const OPEN_BRIEF_GALLERY_VARIABLE: &str = "QUARTERDECK_BRIEF_GALLERY";
/// Where the traffic lights sit: centered in the top bar's 40 px.
const TRAFFIC_LIGHTS_X: f32 = 14.;
const TRAFFIC_LIGHTS_Y: f32 = 13.;

/// Initializes the services, starts gpui and opens the main window where it
/// was last placed, on the display it was on, behind the startup splash when
/// its movie is there. Exits the process when the database cannot be opened.
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
            show_startup_failure(&format!("{error:#}"));
            std::process::exit(1);
        }
    };

    Application::new()
        .with_assets(BundledIcons)
        .run(move |cx: &mut App| {
            install_app_commands(cx);
            load_bundled_fonts(cx);
            let min_size = size(px(WINDOW_MIN_WIDTH), px(WINDOW_MIN_HEIGHT));
            let restored = restore_window(&recorder, min_size, cx);
            cx.set_global(AppSettings::load(&recorder));
            cx.set_global(services.clone());
            cx.set_global(recorder);
            AppUpdater::start(cx);
            AvatarImages::install(cx);

            // Review workspaces left by a crash or a forced quit during a brief
            // go now. Detached: it is quick, and nothing waits for it.
            cx.background_executor()
                .spawn(async {
                    if let Some(root) = workspace_root() {
                        clean_orphan_workspaces(&root);
                    }
                })
                .detach();

            // The main window opens first, hidden during the splash, so the queue
            // loads and refreshes from GitHub while the movie plays.
            let startup = StartupMode::detect();
            let reveal = startup.main_window_reveal();
            let main_window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(restored.bounds),
                        display_id: restored.display_id,
                        window_min_size: Some(min_size),
                        // The app draws the title bar (the queue's top bar); the traffic lights sit in it.
                        titlebar: Some(TitlebarOptions {
                            title: Some(WINDOW_TITLE.into()),
                            appears_transparent: true,
                            traffic_light_position: Some(point(
                                px(TRAFFIC_LIGHTS_X),
                                px(TRAFFIC_LIGHTS_Y),
                            )),
                        }),
                        show: startup.shows_main_window_at_once(),
                        focus: startup.shows_main_window_at_once(),
                        window_background: startup.main_window_background(),
                        ..Default::default()
                    },
                    |window, cx| cx.new(|cx| MainWindow::new(services, reveal, window, cx)),
                )
                .expect("failed to open the main window");
            if let StartupMode::Splash(video) = startup {
                StartupSplash::open(video, main_window, restored.display_id, cx);
            }
            cx.activate(true);

            // While working on brief layouts, the gallery reopens with every launch.
            if std::env::var_os(OPEN_BRIEF_GALLERY_VARIABLE).is_some() {
                open_brief_gallery(cx);
            }
        });
}
