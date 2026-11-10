//! Starting the app: the services, gpui, and the main window where it was
//! last placed, behind the startup splash when its movie is there.
//!
//! Files:
//! - `launch_app.rs`: `launch_app`, starts gpui and opens the main window (and the brief gallery when asked).
//! - `startup_mode.rs`: `StartupMode`, whether the main window comes up behind the splash or at once.
//! - `restore_window.rs`: `restore_window`, where the main window opens: its saved placement, or centered.
//! - `show_startup_failure.rs`: `show_startup_failure`, a system alert saying why the app cannot start.

mod launch_app;
mod restore_window;
mod show_startup_failure;
mod startup_mode;

pub use launch_app::launch_app;
