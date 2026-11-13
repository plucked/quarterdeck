use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::os::unix::fs::{FileTypeExt as _, MetadataExt as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use nix::sys::signal::kill;
use nix::unistd::{Pid, geteuid};
use serde::Deserialize;

use crate::command_output::command_output;
use crate::{ClaudeSession, SessionId, find_claude};

const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_REGISTRY_BYTES: u64 = 65_536;

/// The installed CLI and its registry root. No inventory is cached here:
/// every refresh executes discovery and rereads matching registry records.
#[derive(Clone)]
pub struct SessionInventory {
    pub(crate) executable: PathBuf,
    pub(crate) configuration_directory: PathBuf,
}

/// Public `claude agents --json` fields; exited background jobs may lack a PID.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListedSession {
    pid: Option<u32>,
    session_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    cwd: PathBuf,
    #[serde(default)]
    status: Option<String>,
}

/// Private registry fields verified against the public inventory before use.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegistrySession {
    pid: u32,
    session_id: String,
    proc_start: String,
    messaging_socket_path: PathBuf,
}

impl SessionInventory {
    /// Resolves the same config override as Claude, including Finder launches.
    pub fn discover() -> Result<Self> {
        let executable =
            find_claude().context("Claude Code is not installed. Install the CLI, then refresh")?;
        let configuration_directory = std::env::var_os("CLAUDE_CONFIG_DIR")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| dirs::home_dir().map(|home| home.join(".claude")))
            .context("Could not locate Claude Code's session registry")?;
        Ok(Self {
            executable,
            configuration_directory,
        })
    }

    /// Lists current local sessions with a live PID and a matching inbox.
    /// Changing, missing or incompatible individual records are skipped.
    pub fn refresh(&self) -> Result<Vec<ClaudeSession>> {
        // Ask Claude for its current public inventory; never resume a session.
        let output = command_output(
            Command::new(&self.executable)
                .args(["agents", "--json"])
                .env("CLAUDE_CONFIG_DIR", &self.configuration_directory),
            DISCOVERY_TIMEOUT,
        ).context("Claude Code could not list sessions. Check that `claude agents --json` works with your installed version")?;
        let listed: Vec<ListedSession> = serde_json::from_slice(&output).context(
            "Claude Code returned an unsupported session inventory. Update the CLI and refresh",
        )?;
        let processes = process_starts()?;

        // A single stale or half-written registry record cannot hide other sessions.
        let mut sessions: Vec<_> = listed
            .into_iter()
            .filter_map(|listed| self.join_registry(listed, &processes).ok())
            .collect();
        sessions.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.id.as_str().cmp(right.id.as_str()))
        });
        sessions.dedup_by(|left, right| left.same_process(right));
        Ok(sessions)
    }

    /// Matches identity across sources and checks local process/socket ownership.
    fn join_registry(
        &self,
        listed: ListedSession,
        processes: &HashMap<u32, String>,
    ) -> Result<ClaudeSession> {
        let process_id = listed.pid.context("Session has no live process")?;
        let raw_process_id = i32::try_from(process_id)?;
        ensure!(raw_process_id > 0, "Invalid session process");
        kill(Pid::from_raw(raw_process_id), None).context("Session process ended")?;
        let id = SessionId::parse(listed.session_id.context("Session has no UUID")?)?;

        // Numeric PIDs form paths; neither names nor UUIDs can choose a file.
        let registry_path = self
            .configuration_directory
            .join("sessions")
            .join(format!("{process_id}.json"));
        let metadata = fs::symlink_metadata(&registry_path)?;
        ensure!(
            metadata.file_type().is_file() && metadata.uid() == geteuid().as_raw(),
            "Session registry is not an owned regular file"
        );
        let mut bytes = Vec::new();
        fs::File::open(registry_path)?
            .take(MAX_REGISTRY_BYTES + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= MAX_REGISTRY_BYTES,
            "Session registry is too large"
        );
        let registry: RegistrySession = serde_json::from_slice(&bytes)?;
        ensure!(
            registry.pid == process_id
                && registry.session_id == id.as_str()
                && !registry.proc_start.trim().is_empty(),
            "Session identity changed"
        );
        let process_start = registry
            .proc_start
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        ensure!(
            processes.get(&process_id) == Some(&process_start),
            "Session process was replaced"
        );
        check_socket(&registry.messaging_socket_path)?;

        let name = listed
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Unnamed session".into());
        Ok(ClaudeSession {
            id,
            name,
            working_directory: listed.cwd,
            status: listed.status.unwrap_or_else(|| "Unknown activity".into()),
            process_id,
            process_start,
            socket_path: registry.messaging_socket_path,
        })
    }
}

/// A single OS snapshot prevents stale registry records surviving PID reuse.
/// Claude records `procStart` using C locale and UTC, regardless of the app's
/// timezone. Both must match before the strings can identify the same process.
fn process_starts() -> Result<HashMap<u32, String>> {
    let output = command_output(
        Command::new("/bin/ps")
            .args(["-axo", "pid=,lstart="])
            .env("LC_ALL", "C")
            .env("TZ", "UTC"),
        DISCOVERY_TIMEOUT,
    )?;
    let text = std::str::from_utf8(&output).context("Could not read local process identities")?;
    Ok(text
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let process_id = fields.next()?.parse().ok()?;
            Some((process_id, fields.collect::<Vec<_>>().join(" ")))
        })
        .collect())
}

/// Requires an owned Unix inbox, rejecting symlinks and ordinary files.
pub(crate) fn check_socket(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), "Claude inbox path is not absolute");
    let metadata = fs::symlink_metadata(path)
        .context("Claude inbox is no longer available. Refresh the sessions")?;
    ensure!(
        metadata.file_type().is_socket() && metadata.uid() == geteuid().as_raw(),
        "Claude inbox is not an owned Unix socket"
    );
    Ok(())
}
