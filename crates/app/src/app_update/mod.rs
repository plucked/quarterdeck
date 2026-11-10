//! The app's self-update: checks GitHub releases in the background,
//! downloads and verifies a newer version, and installs it on quit or on a
//! restart; `qd-update` does the work, this module schedules it and keeps
//! the status views show.
//!
//! Files:
//! - `app_updater.rs`: `AppUpdater`, the global entity running checks, downloads and installs.
//! - `update_status.rs`: `UpdateStatus` and `ReadyUpdate`, where the update stands.
//! - `update_decision.rs`: `decide_update`, what a check does with the newest release.
//! - `restart_app.rs`: `restart_app`, quits and starts the app again (an installed update, a new token).

mod app_updater;
mod restart_app;
mod update_decision;
mod update_status;

pub use app_updater::AppUpdater;
pub use restart_app::restart_app;
pub use update_status::UpdateStatus;
