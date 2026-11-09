use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A new, empty folder under `parent` named `<prefix><pid>-<nanoseconds>`:
/// this process's id and the time, so it is unique and an orphan of a dead
/// process can be told apart (`clean_orphan_workspaces` reads the pid).
pub(crate) fn new_process_folder(parent: &Path, prefix: &str) -> std::io::Result<PathBuf> {
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let folder = parent.join(format!("{prefix}{}-{started}", std::process::id()));
    std::fs::create_dir_all(&folder)?;
    Ok(folder)
}
