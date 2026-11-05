//! quick-check: a macOS desktop app for reviewing GitHub pull requests.
//!
//! Files:
//! - `launch_app.rs`: starts gpui and opens the main window where it was last placed.
//! - `main_window.rs`: `MainWindow`, the root view; records the window placement.
//! - `app_commands.rs`: app-level actions with their key bindings and menus.
//! - `app_services.rs`: `AppServices`, the GitHub client and cache handed to views.
//! - `scrollbar/`: `VerticalScrollbar`, a draggable scrollbar for any scroll container.
//! - `shared_cache.rs`: `SharedCache`, the cache shared between views and background tasks.
//! - `theme.rs`: the color palette and fonts.
//! - `markdown_view/`: `MarkdownView`, renders parsed Markdown (descriptions, comments).
//! - `review_queue/`: the main view, the queue of pull requests awaiting review.
//! - `ui_state/`: saves and restores what the UI showed across relaunches.

mod app_commands;
mod app_services;
mod launch_app;
mod main_window;
mod markdown_view;
mod review_queue;
mod scrollbar;
mod shared_cache;
mod theme;
mod ui_state;

fn main() {
    launch_app::launch_app();
}
