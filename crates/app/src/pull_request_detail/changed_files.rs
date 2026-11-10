use gpui::SharedString;

use super::load_progress::LoadProgress;
use super::loaded_file::LoadedFile;

/// The load state of the shown files.
pub enum ChangedFiles {
    /// Being read from the cache or fetched from GitHub; how far it came.
    Loading(LoadProgress),
    /// The files with their diffs built for the current settings.
    Loaded(Vec<LoadedFile>),
    /// Loading failed; the message is shown to the user.
    Failed(SharedString),
}

impl ChangedFiles {
    /// The loaded files; none while loading or after a failure.
    pub fn loaded(&self) -> &[LoadedFile] {
        match self {
            Self::Loaded(files) => files,
            Self::Loading(_) | Self::Failed(_) => &[],
        }
    }

    /// The loaded files to change in place, once loaded.
    pub fn loaded_mut(&mut self) -> Option<&mut Vec<LoadedFile>> {
        match self {
            Self::Loaded(files) => Some(files),
            Self::Loading(_) | Self::Failed(_) => None,
        }
    }

    /// How far the load has come, while it runs.
    pub fn progress(&self) -> Option<LoadProgress> {
        match self {
            Self::Loading(progress) => Some(*progress),
            Self::Loaded(_) | Self::Failed(_) => None,
        }
    }

    /// Why the load failed, if it did.
    pub fn failure(&self) -> Option<&SharedString> {
        match self {
            Self::Failed(message) => Some(message),
            Self::Loading(_) | Self::Loaded(_) => None,
        }
    }
}
