//! quick-check: a macOS desktop app for reviewing GitHub pull requests.
//!
//! Files:
//! - `launch_app.rs`: starts gpui and opens the main window.
//! - `app_commands.rs`: app-level actions with their key bindings and menus.
//! - `app_services.rs`: `AppServices`, the GitHub client and cache handed to views.
//! - `shared_cache.rs`: `SharedCache`, the cache shared between views and background tasks.
//! - `theme.rs`: the color palette and fonts.
//! - `review_queue/`: the main view, the queue of pull requests awaiting review.

mod app_commands;
mod app_services;
mod launch_app;
mod review_queue;
mod shared_cache;
mod theme;

fn main() {
    launch_app::launch_app();
}
