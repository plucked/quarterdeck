use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail, ensure};
use wait_timeout::ChildExt as _;

/// Local discovery output is small; reject a broken executable flooding stdout.
const MAX_OUTPUT_BYTES: u64 = 1_048_576;

/// Runs discovery without stdin, bounding its runtime and buffered output.
pub(crate) fn command_output(command: &mut Command, timeout: Duration) -> Result<Vec<u8>> {
    let deadline = Instant::now() + timeout;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("Could not start the local discovery command")?;

    // Drain concurrently so a full pipe cannot keep the child from exiting.
    let stdout = child
        .stdout
        .take()
        .context("Discovery command stdout is unavailable")?;
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let result = stdout
            .take(MAX_OUTPUT_BYTES + 1)
            .read_to_end(&mut output)
            .map(|_| output);
        let _ = sender.send(result);
    });

    // Reap the child on every wait failure, including a hung CLI.
    let status = match child.wait_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(Some(status)) => status,
        result => {
            let _ = child.kill();
            let _ = child.wait();
            result.context("Could not wait for the discovery command")?;
            bail!("Local discovery command timed out. Refresh to try again");
        }
    };
    ensure!(
        status.success(),
        "Local discovery command exited with {status}"
    );
    let output = receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .context("Local discovery command output timed out")??;
    ensure!(
        output.len() as u64 <= MAX_OUTPUT_BYTES,
        "Local discovery command output is too large"
    );
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A silent discovery process is killed rather than blocking the review UI indefinitely.
    #[test]
    fn silent_process_has_a_deadline() {
        let start = Instant::now();
        let result = command_output(
            Command::new("/bin/sleep").arg("30"),
            Duration::from_millis(50),
        );
        assert!(result.unwrap_err().to_string().contains("timed out"));
        assert!(start.elapsed() < Duration::from_secs(3));
    }
}
