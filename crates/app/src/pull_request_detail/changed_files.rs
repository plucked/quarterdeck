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
