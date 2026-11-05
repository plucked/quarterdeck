use gpui::SharedString;
use qc_github::PullRequestFile;

/// The load state of the selected pull request's changed files.
pub enum ChangedFiles {
    /// Being read from the cache or fetched from GitHub.
    Loading,
    Loaded(Vec<PullRequestFile>),
    /// Loading failed; the message is shown to the user.
    Failed(SharedString),
}
