use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use socket2::{Domain, SockAddr, Socket, Type};

use crate::session_inventory::check_socket;
use crate::{ClaudeSession, SessionInventory};

const SOCKET_TIMEOUT: Duration = Duration::from_secs(3);
/// A byte cap is conservative relative to Claude's 1 Mi UTF-16-unit buffer.
const MAX_FRAME_BYTES: usize = 1_048_576;
const ATTRIBUTION: &str = "Hey! These review notes were sent by a user using Quarterdeck, the pull request reviewing tool. Quarterdeck can't receive replies, so please don't try to message it back. Any questions or updates need to stay here in Claude Code for the user to see.";

/// Submits one-way messages to existing sessions; never claims delivery or
/// execution, supplies a reply endpoint, or attests a sender permission mode.
pub struct SessionSender {
    inventory: SessionInventory,
}

impl SessionSender {
    pub fn new(inventory: SessionInventory) -> Self {
        Self { inventory }
    }

    /// Refreshes immediately before connecting and refuses an ended, restarted
    /// or resumed process. A successful return means only that bytes were written.
    pub fn send(&self, selected: &ClaudeSession, message: &str) -> Result<()> {
        // Validate and serialize before discovery so a large review cannot age it.
        let frame = message_frame(selected, message)?;
        let sessions = self.inventory.refresh()?;
        let current = sessions
            .iter()
            .find(|session| selected.same_process(session))
            .context("That Claude session ended or changed. Refresh and choose a session again")?;
        check_socket(&current.socket_path)?;

        // The live peer must be the process the user selected, even if a socket
        // path was replaced between inventory refresh and connect.
        let socket = Socket::new(Domain::UNIX, Type::STREAM, None)?;
        socket
            .connect_timeout(&SockAddr::unix(&current.socket_path)?, SOCKET_TIMEOUT)
            .context("Could not connect to Claude. Refresh the sessions and try again")?;
        let descriptor: std::os::fd::OwnedFd = socket.into();
        let mut stream = UnixStream::from(descriptor);
        check_peer(&stream, current)?;

        // Bound the whole write, including a reader that consumes tiny chunks.
        // Do not retry a partial write: that could submit a duplicate review.
        let deadline = Instant::now() + SOCKET_TIMEOUT;
        let mut remaining = frame.as_slice();
        while !remaining.is_empty() {
            let timeout = deadline.saturating_duration_since(Instant::now());
            ensure!(
                !timeout.is_zero(),
                "Claude submission timed out. Check Claude before trying again"
            );
            stream.set_write_timeout(Some(timeout))?;
            let written = stream
                .write(remaining)
                .context("Claude submission failed. Check Claude before trying again")?;
            ensure!(
                written != 0,
                "Claude closed its inbox. Check Claude before trying again"
            );
            remaining = &remaining[written..];
        }
        stream
            .shutdown(Shutdown::Write)
            .context("Could not finish the submission. Check Claude before trying again")?;
        Ok(())
    }
}

/// Serializes a single newline-delimited user frame, with attribution last.
fn message_frame(session: &ClaudeSession, message: &str) -> Result<Vec<u8>> {
    ensure!(
        !message.trim().is_empty(),
        "There are no review notes to send"
    );
    let content = format!("{message}\n\n---\n\n{ATTRIBUTION}");
    let mut frame = serde_json::to_vec(&serde_json::json!({
        "type": "user",
        "session_id": session.id.as_str(),
        "message": {"role": "user", "content": content},
    }))?;
    frame.push(b'\n');
    ensure!(
        frame.len() <= MAX_FRAME_BYTES,
        "These notes are too large for Claude's inbox. Copy them instead or send fewer notes"
    );
    Ok(frame)
}

/// Uses OS peer identity rather than trusting the filesystem path alone.
fn check_peer(stream: &UnixStream, session: &ClaudeSession) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use nix::sys::socket::{getsockopt, sockopt::LocalPeerPid};
        use nix::unistd::getpeereid;
        let process_id = getsockopt(stream, LocalPeerPid)?;
        let (user_id, _) = getpeereid(stream)?;
        ensure!(
            u32::try_from(process_id)? == session.process_id && user_id == nix::unistd::geteuid(),
            "Claude inbox belongs to a different process. Refresh the sessions"
        );
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (stream, session);
        anyhow::bail!("Sending to Claude sessions is supported on macOS only");
    }
    Ok(())
}

#[cfg(test)]
mod tests;
