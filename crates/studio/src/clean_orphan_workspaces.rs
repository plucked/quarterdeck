use std::path::Path;
use std::process::{Command, Stdio};

use super::prepare_workspace::WORKSPACE_PREFIX;

/// Removes the review workspaces under `root` whose app process is gone (a
/// crash or a forced quit during a run); a running app's workspaces (a
/// release and a development build share the folder) are left alone.
/// Returns how many were removed.
pub fn clean_orphan_workspaces(root: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        // Only folders named `workspace-<pid>-<time>` are ours.
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|name| name.strip_prefix(WORKSPACE_PREFIX))
            .and_then(|rest| rest.split('-').next())
            .and_then(|pid| pid.parse::<u32>().ok())
        else {
            continue;
        };
        if !is_process_running(pid) && std::fs::remove_dir_all(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Whether a process with `pid` exists: `kill -0` sends no signal and
/// succeeds only for a live process.
fn is_process_running(pid: u32) -> bool {
    pid == std::process::id()
        || Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
}
