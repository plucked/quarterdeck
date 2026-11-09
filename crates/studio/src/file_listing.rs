use std::sync::Arc;

use qd_diff::FileDiff;

/// What the agent gets to see of one changed file.
#[derive(Clone)]
pub enum FileListing {
    /// The file's diff, numbered line by line.
    Diff(Arc<FileDiff>),
    /// Only its name and size, for the reason given (generated, binary, too large).
    LeftOut(String),
}
