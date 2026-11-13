use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::command_output::command_output;
use crate::git_command::git_command;

const REPOSITORY_TIMEOUT: Duration = Duration::from_secs(2);

/// The main checkout path shared by a local repository and all its linked worktrees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitRepository(PathBuf);

impl GitRepository {
    /// Reads Git's main-worktree record, including from nested or detached worktrees.
    pub fn read(directory: &Path) -> Option<Self> {
        // Git lists the main worktree first; NUL records preserve unusual path names.
        let output = command_output(
            git_command(directory).args(["worktree", "list", "--porcelain", "-z"]),
            REPOSITORY_TIMEOUT,
        )
        .ok()?;
        let path = output
            .split(|byte| *byte == 0)
            .next()?
            .strip_prefix(b"worktree ")?;
        let path = PathBuf::from(OsStr::from_bytes(path));
        if !path.is_absolute() {
            return None;
        }
        // Symlink spellings share a group; a missing main checkout still has an identity.
        Some(Self(path.canonicalize().unwrap_or(path)))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}
