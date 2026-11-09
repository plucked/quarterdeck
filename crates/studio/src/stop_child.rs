use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// How long a process asked to stop may take to exit before it is killed.
const STOP_GRACE: Duration = Duration::from_secs(2);
/// How often the grace period looks whether the process has exited (std
/// has no wait with a timeout).
const EXIT_CHECK_INTERVAL: Duration = Duration::from_millis(50);

/// Stops `child` politely, then by force: `SIGTERM` (Claude Code and git
/// end their requests and exit), up to `STOP_GRACE` for it to exit, then
/// `SIGKILL`; reaped either way. Safe from pid reuse: the child is not
/// reaped before the signals, so its pid is still its own.
pub(crate) fn stop_child(child: &mut Child) {
    // Ask it to stop.
    let _ = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // Give it the grace period to exit.
    let deadline = Instant::now() + STOP_GRACE;
    while Instant::now() < deadline {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) => std::thread::sleep(EXIT_CHECK_INTERVAL),
            Err(_) => break,
        }
    }

    // Then end it.
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A process that exits on `SIGTERM` stops at once; one that ignores it
    /// is killed once the grace period is over.
    #[test]
    fn stops_politely_then_by_force() {
        let mut polite = Command::new("sleep").arg("30").spawn().expect("sleep");
        let started = Instant::now();
        stop_child(&mut polite);
        assert!(started.elapsed() < STOP_GRACE);

        let mut stubborn = Command::new("sh")
            .args(["-c", "trap '' TERM; sleep 30"])
            .spawn()
            .expect("sh");
        // Give the shell time to install its trap.
        std::thread::sleep(Duration::from_millis(200));
        let started = Instant::now();
        stop_child(&mut stubborn);
        let took = started.elapsed();
        assert!(took >= STOP_GRACE && took < STOP_GRACE * 2, "{took:?}");
        assert!(stubborn.try_wait().expect("reaped").is_some());
    }
}
