use gpui::SharedString;

use super::loaded_file::LoadedFile;

/// The load state of the selected pull request's changed files.
pub enum ChangedFiles {
    /// Being read from the cache or fetched from GitHub.
    Loading,
    /// The files with their diffs built for the current settings.
    Loaded(Vec<LoadedFile>),
    /// Loading failed; the message is shown to the user.
    Failed(SharedString),
}
