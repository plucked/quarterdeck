//! Self-update of the app bundle: reads the newest release from GitHub,
//! downloads it, checks it and puts it in place of the running bundle.
//!
//! Blocking and UI-free; the app runs it on gpui's background executor and
//! shows its progress.
//!
//! Files:
//! - `app_version.rs`: `AppVersion`, `major.minor.patch`, compared by number.
//! - `release_manifest.rs`: `ReleaseManifest`, the newest release's `latest.json`, checked.
//! - `update_feed.rs`: `UpdateFeed`, fetches the newest release's manifest.
//! - `download_release.rs`: `UpdateFeed::download_release`, the zip, size- and SHA-256-checked.
//! - `update_downloads.rs`: `update_download_directory`, where downloads are kept.
//! - `code_signature.rs`: the release's signature requirement (Developer ID of the team) and Gatekeeper check.
//! - `running_bundle.rs`: `RunningBundle`, the running app's bundle: unpacks, verifies and installs a release.
//! - `staged_update.rs`: `StagedUpdate`, a verified release unpacked beside the bundle.
//! - `update_blocker.rs`: `UpdateBlocker`, why the running app cannot update itself.
//! - `relaunch_after_exit.rs`: starts the app again once this process has exited.
//! - `update_error.rs`: `UpdateError`, every way an update can fail.

mod app_version;
mod code_signature;
mod download_release;
mod relaunch_after_exit;
mod release_manifest;
mod running_bundle;
mod staged_update;
mod update_blocker;
mod update_downloads;
mod update_error;
mod update_feed;

pub use app_version::AppVersion;
pub use relaunch_after_exit::relaunch_after_exit;
pub use release_manifest::ReleaseManifest;
pub use running_bundle::RunningBundle;
pub use staged_update::StagedUpdate;
pub use update_blocker::UpdateBlocker;
pub use update_downloads::update_download_directory;
pub use update_error::UpdateError;
pub use update_feed::UpdateFeed;
