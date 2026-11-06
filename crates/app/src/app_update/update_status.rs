use std::sync::Arc;

use gpui::SharedString;
use qd_markdown::MarkdownDocument;
use qd_update::{AppVersion, ReleaseManifest, StagedUpdate, UpdateBlocker};

/// Where the app's self-update stands.
#[derive(Clone)]
pub enum UpdateStatus {
    /// Finding the running bundle and its signature, at launch.
    Starting,
    /// The running app cannot update itself, and why.
    Unavailable(UpdateBlocker),
    /// Waiting for the next check; the last one found nothing newer (or
    /// none ran yet).
    UpToDate,
    /// Reading the newest release's manifest.
    Checking,
    /// Downloading and verifying this version.
    Downloading(AppVersion),
    /// A newer version is unpacked and verified, ready to install.
    Ready(ReadyUpdate),
    /// Putting the new version in place before a restart.
    Installing,
    /// The last check, download or install failed; the next check retries.
    Failed(SharedString),
}

/// A verified release waiting to be installed, with its notes parsed for display.
#[derive(Clone)]
pub struct ReadyUpdate {
    pub release: ReleaseManifest,
    pub staged: StagedUpdate,
    pub notes: Arc<MarkdownDocument>,
}

impl UpdateStatus {
    /// The status in one line, for the settings page.
    pub fn describe(&self) -> SharedString {
        match self {
            Self::Starting => "Looking for updates soon.".into(),
            Self::Unavailable(blocker) => blocker.describe().into(),
            Self::UpToDate => "Quarterdeck is up to date.".into(),
            Self::Checking => "Checking for updates…".into(),
            Self::Downloading(version) => format!("Downloading version {version}…").into(),
            Self::Ready(ready) => format!(
                "Version {} is ready. It installs when you quit Quarterdeck, or now with a restart.",
                ready.release.version
            )
            .into(),
            Self::Installing => "Installing the update…".into(),
            Self::Failed(reason) => format!("The last update check failed: {reason}").into(),
        }
    }

    /// Whether a check can start now: the app can update itself and no
    /// check, download or install runs.
    pub fn can_check(&self) -> bool {
        matches!(self, Self::UpToDate | Self::Ready(_) | Self::Failed(_))
    }
}
