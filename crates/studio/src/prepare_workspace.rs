use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use super::new_process_folder::new_process_folder;
use super::output_lines::{NextLine, OutputLines};
use super::review_workspace::ReviewWorkspace;
use super::stop_child::stop_child;
use super::text_tail::tail;
use super::workspace_error::WorkspaceError;
use super::workspace_progress::WorkspaceProgress;
use super::workspace_source::WorkspaceSource;

/// The start of a workspace folder's name; the app's process id and the
/// time follow, so orphans can be told from a running app's.
pub(crate) const WORKSPACE_PREFIX: &str = "workspace-";
/// The folder holding the repository at the pull request's head.
pub const HEAD_FOLDER: &str = "head";
/// The pull request's diff.
pub const DIFF_FILE: &str = "change.diff";
/// The temporary bare repository the commit is fetched into; removed
/// before the agent starts.
const STORE_FOLDER: &str = ".store";
/// How much of git's error output is kept for the error message.
const MAX_ERROR_OUTPUT_CHARS: usize = 1_500;

/// Prepares a review workspace under `root` for `source`: fetches only the
/// head commit (depth 1, no tags, Git LFS files left as pointers) into a
/// temporary bare repository, writes its files to `head/` (no `.git`, so
/// the agent sees plain files), writes `change_diff` (the pull request's
/// diff, which shows what each changed file was before) to `change.diff`,
/// and removes the repository. The token goes to git as an
/// `http.extraheader` in the environment, so it is in no file, URL or
/// argument; the user's git configuration is ignored, so no credential
/// helper, hook or URL rewrite takes part. Blocks; setting `cancel` stops
/// the download within a fifth of a second, even while git prints nothing.
/// A failure removes the folder.
pub fn prepare_workspace(
    root: &Path,
    source: &WorkspaceSource,
    change_diff: &str,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(WorkspaceProgress),
) -> Result<ReviewWorkspace, WorkspaceError> {
    // A new folder; owning it now removes it on any early return.
    let workspace = ReviewWorkspace::owning(new_process_folder(root, WORKSPACE_PREFIX)?);
    let store = workspace.path().join(STORE_FOLDER);
    let git = Git {
        store: store.clone(),
        token: source.token.clone(),
    };

    // An empty repository, then the head commit fetched into it.
    run(
        git.command_without_store()
            .args(["init", "--bare", "--quiet"])
            .arg(&store),
        "init",
    )?;
    on_progress(WorkspaceProgress::Fetching { percent: None });
    let fetch = git
        .command()
        .args(["fetch", "--depth", "1", "--no-tags", "--progress"])
        .arg(&source.remote_url)
        .arg(&source.head_sha)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    follow_fetch(fetch, cancel, on_progress)?;

    // The commit's files and the diff, and the repository gone.
    on_progress(WorkspaceProgress::Unpacking);
    unpack(&git, &source.head_sha, &workspace.path().join(HEAD_FOLDER))?;
    std::fs::write(workspace.path().join(DIFF_FILE), change_diff)?;
    std::fs::remove_dir_all(&store)?;
    Ok(workspace)
}

/// Builds git commands against the temporary repository.
struct Git {
    store: PathBuf,
    token: Option<String>,
}

impl Git {
    /// `git --git-dir <store>`, as `command_without_store` sets it up.
    fn command(&self) -> Command {
        let mut command = self.command_without_store();
        command.arg("--git-dir").arg(&self.store);
        command
    }

    /// `git` with a predictable environment: no prompts, LFS smudging off,
    /// system and user configuration ignored, and the token (when given) as
    /// an authorization header for github.com only.
    fn command_without_store(&self) -> Command {
        let mut command = Command::new("git");
        command
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_LFS_SKIP_SMUDGE", "1")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .stdin(Stdio::null());
        if let Some(token) = &self.token {
            let credentials = STANDARD.encode(format!("x-access-token:{token}"));
            command
                .env("GIT_CONFIG_COUNT", "1")
                .env("GIT_CONFIG_KEY_0", "http.https://github.com/.extraheader")
                .env(
                    "GIT_CONFIG_VALUE_0",
                    format!("Authorization: Basic {credentials}"),
                );
        }
        command
    }
}

/// Follows a fetch's progress output (lines ending in `\r` or `\n`),
/// reporting the percentage of each stage and stopping it (`stop_child`)
/// when `cancel` is set, also while git prints nothing; an error with
/// git's last words when it fails.
fn follow_fetch(
    mut fetch: Child,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(WorkspaceProgress),
) -> Result<(), WorkspaceError> {
    let mut output = String::new();
    if let Some(stderr) = fetch.stderr.take() {
        let lines = OutputLines::read(stderr);
        loop {
            match lines.next(cancel) {
                // Each line may carry a percentage.
                NextLine::Line(text) => {
                    if let Some(percent) = progress_percent(&text) {
                        on_progress(WorkspaceProgress::Fetching {
                            percent: Some(percent),
                        });
                    }
                    output.push_str(&text);
                    output.push('\n');
                }
                NextLine::Ended => break,
                NextLine::Cancelled => {
                    stop_child(&mut fetch);
                    return Err(WorkspaceError::Cancelled);
                }
            }
        }
    }
    let status = fetch.wait()?;
    if status.success() {
        return Ok(());
    }
    Err(WorkspaceError::Git {
        step: "fetch",
        message: tail(output.trim(), MAX_ERROR_OUTPUT_CHARS),
    })
}

/// The percentage in a git progress line such as `Receiving objects:  45%
/// (1234/2742), 1.2 MiB`, for the receiving stage (the long one).
fn progress_percent(line: &str) -> Option<u8> {
    let rest = line.trim_start().strip_prefix("Receiving objects:")?;
    rest.trim_start()
        .split('%')
        .next()?
        .trim()
        .parse::<u8>()
        .ok()
}

/// Writes the files of commit `sha` into `folder`: `git archive` piped
/// into `tar`, so no `.git` comes along.
fn unpack(git: &Git, sha: &str, folder: &Path) -> Result<(), WorkspaceError> {
    std::fs::create_dir_all(folder)?;
    let mut archive = git
        .command()
        .args(["archive", "--format=tar", sha])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let archive_output = archive.stdout.take().map_or_else(Stdio::null, Stdio::from);
    let extracted = run(
        Command::new("tar")
            .arg("-x")
            .arg("-C")
            .arg(folder)
            .stdin(archive_output),
        "unpack",
    );
    let archived = archive.wait_with_output()?;
    if !archived.status.success() {
        return Err(WorkspaceError::Git {
            step: "archive",
            message: tail(
                String::from_utf8_lossy(&archived.stderr).trim(),
                MAX_ERROR_OUTPUT_CHARS,
            ),
        });
    }
    extracted
}

/// Runs `command` to the end; an error naming `step` with its error output
/// when it fails.
fn run(command: &mut Command, step: &'static str) -> Result<(), WorkspaceError> {
    let output = command.stderr(Stdio::piped()).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(WorkspaceError::Git {
        step,
        message: tail(
            String::from_utf8_lossy(&output.stderr).trim(),
            MAX_ERROR_OUTPUT_CHARS,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The receiving stage's percentage is read; other lines give none.
    #[test]
    fn reads_receiving_percentages() {
        assert_eq!(
            progress_percent("Receiving objects:  45% (1234/2742), 1.20 MiB | 3.1 MiB/s"),
            Some(45)
        );
        assert_eq!(
            progress_percent("remote: Counting objects: 12% (1/8)"),
            None
        );
        assert_eq!(progress_percent("Resolving deltas: 100% (3/3)"), None);
    }
}
