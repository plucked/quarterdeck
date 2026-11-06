use std::process::{Command, Stdio};

use crate::running_bundle::bundle_of;

/// Waits for the process whose id is `$0` to exit, then opens `$1` (an app
/// bundle, through Launch Services).
const OPEN_AFTER_EXIT: &str =
    r#"while /bin/kill -0 "$0" 2>/dev/null; do /bin/sleep 0.1; done; exec /usr/bin/open "$1""#;
/// The same for a bare executable (`$1` runs directly).
const RUN_AFTER_EXIT: &str =
    r#"while /bin/kill -0 "$0" 2>/dev/null; do /bin/sleep 0.1; done; exec "$1""#;

/// Starts the app again once this process has exited: its bundle (which
/// may have been replaced by an update meanwhile) or, outside a bundle,
/// its executable. The caller quits the app right after.
pub fn relaunch_after_exit() -> std::io::Result<()> {
    // What to start: the bundle the executable sits in, else the executable itself.
    let executable = std::env::current_exe()?;
    let (script, target) = match bundle_of(&executable) {
        Some(bundle) => (OPEN_AFTER_EXIT, bundle),
        None => (RUN_AFTER_EXIT, executable),
    };

    // A detached shell waits for this process to end; it outlives it.
    Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg(std::process::id().to_string())
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}
