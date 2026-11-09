use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

/// How often a wait for the next line looks at the cancel flag while the
/// process prints nothing (a stalled download, a silent model).
const CANCEL_CHECK_INTERVAL: Duration = Duration::from_millis(200);

/// What waiting for a process's next output line gave.
pub(crate) enum NextLine {
    Line(String),
    /// The output ended: the process closed it or exited.
    Ended,
    /// The cancel flag was set while waiting.
    Cancelled,
}

/// A child process's output as lines, read on a thread of its own so a
/// wait for the next one can also notice a cancelled run while the process
/// is silent. Lines end at `\n` or `\r` (git rewrites its progress line
/// with `\r`; JSON lines never hold a raw `\r`).
pub(crate) struct OutputLines {
    lines: Receiver<String>,
}

impl OutputLines {
    /// Starts reading `output`; the thread ends with the output.
    pub(crate) fn read(mut output: impl Read + Send + 'static) -> Self {
        let (sender, lines) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            let mut line = Vec::new();
            loop {
                let read = match output.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => read,
                };
                for &byte in &buffer[..read] {
                    if byte == b'\n' || byte == b'\r' {
                        let text = String::from_utf8_lossy(&line).into_owned();
                        line.clear();
                        if sender.send(text).is_err() {
                            return;
                        }
                    } else {
                        line.push(byte);
                    }
                }
            }
            // A last line without its end still counts.
            if !line.is_empty() {
                let _ = sender.send(String::from_utf8_lossy(&line).into_owned());
            }
        });
        Self { lines }
    }

    /// The next line, or that the output ended, or that `cancel` was set
    /// meanwhile (looked at every `CANCEL_CHECK_INTERVAL` while waiting).
    pub(crate) fn next(&self, cancel: &AtomicBool) -> NextLine {
        loop {
            if cancel.load(Ordering::Relaxed) {
                return NextLine::Cancelled;
            }
            match self.lines.recv_timeout(CANCEL_CHECK_INTERVAL) {
                Ok(line) => return NextLine::Line(line),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return NextLine::Ended,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lines split at `\n` and `\r`, and a last line without its end is kept.
    #[test]
    fn splits_lines_at_both_ends() {
        let lines = OutputLines::read(&b"one\ntwo\rthree"[..]);
        let cancel = AtomicBool::new(false);
        let mut read = Vec::new();
        while let NextLine::Line(line) = lines.next(&cancel) {
            read.push(line);
        }
        assert_eq!(read, ["one", "two", "three"]);
    }

    /// A set cancel flag ends the wait even when no line comes.
    #[test]
    fn notices_a_cancel_while_silent() {
        let (reader, _writer) = std::io::pipe().expect("a pipe");
        let lines = OutputLines::read(reader);
        let cancel = AtomicBool::new(true);
        assert!(matches!(lines.next(&cancel), NextLine::Cancelled));
    }
}
