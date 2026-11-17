/// A failed generation or a completed artifact still waiting for durable storage.
#[derive(Debug)]
pub(super) enum ReviewJobError<Draft> {
    Failed(String),
    SaveFailed { draft: Box<Draft>, message: String },
}

impl<Draft> From<String> for ReviewJobError<Draft> {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}
