use std::path::PathBuf;

use crate::SessionId;

/// A sendable session from one inventory refresh. Names are display-only;
/// the UUID, PID and process start marker identify the intended receiver.
#[derive(Clone, Debug)]
pub struct ClaudeSession {
    pub id: SessionId,
    pub name: String,
    pub working_directory: PathBuf,
    pub status: String,
    pub(crate) process_id: u32,
    pub(crate) process_start: String,
    pub(crate) socket_path: PathBuf,
}

impl ClaudeSession {
    /// Whether a new inventory record still describes the process selected.
    pub(crate) fn same_process(&self, other: &Self) -> bool {
        self.id == other.id
            && self.process_id == other.process_id
            && self.process_start == other.process_start
    }
}
