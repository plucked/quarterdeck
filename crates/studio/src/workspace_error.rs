/// Why a review workspace could not be prepared.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("could not prepare the folder: {0}")]
    Io(#[from] std::io::Error),
    #[error("git {step} failed: {message}")]
    Git { step: &'static str, message: String },
    #[error("preparing the code was cancelled")]
    Cancelled,
}
