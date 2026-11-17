/// The local save phase of an accepted brief, separate from agent generation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BriefSaveStatus {
    /// No accepted brief is waiting to be saved or being saved.
    #[default]
    None,
    /// Saving failed; the accepted brief remains available for retry or discard.
    Pending,
    /// The accepted brief is being saved without running the agent again.
    Saving,
}
