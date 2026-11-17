use std::io::{BufRead as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use qd_agent_mcp::McpServer;
use serde_json::Value;

use crate::claude_event::ClaudeEvent;
use crate::mcp_model::McpModel;
use crate::mcp_run_format::{MCP_COMPLETION_SCHEMA, McpRunFormat};
use crate::new_process_folder::new_process_folder;
use crate::run_mcp_handler::RunMcpHandler;
use crate::text_tail::tail;
use crate::{AgentAnswer, AgentProgress, AgentRunError};

const CANCEL_INTERVAL: Duration = Duration::from_millis(200);
const TERMINAL_EXIT_GRACE: Duration = Duration::from_secs(10);
const MAX_STREAM_BYTES: u64 = 32 * 1024 * 1024;
const PIPE_COMPLETION_GRACE: Duration = Duration::from_secs(2);
const STOP_GRACE: Duration = Duration::from_secs(2);
const MAX_ERROR_OUTPUT_CHARS: usize = 2_000;
const ISOLATION_SETTINGS: &str =
    r#"{"claudeMdExcludes":["**"],"disableAllHooks":true,"autoMemoryEnabled":false}"#;

/// Stops the Claude child even if stream handling returns early or unwinds.
struct McpChild {
    child: Child,
    reaped: bool,
}

impl Drop for McpChild {
    fn drop(&mut self) {
        if !self.reaped {
            stop_mcp_child(&mut self.child);
        }
    }
}

/// Removes only the empty working folder this run itself created.
struct McpRunFolder(Option<PathBuf>);

impl Drop for McpRunFolder {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

/// Owns one MCP endpoint and Claude process; artifact-specific runners verify its receipt.
pub(crate) fn run_mcp_review<H: RunMcpHandler + 'static>(
    program: &Path,
    prompt: &str,
    workspace: Option<&Path>,
    format: McpRunFormat<'_>,
    handler: &Arc<H>,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
) -> Result<AgentAnswer, AgentRunError> {
    // Fail before opening an endpoint when inherited modes would suppress MCP.
    check_environment()?;
    if cancel.load(Ordering::Relaxed) {
        return Err(AgentRunError::Cancelled);
    }
    let folder = match workspace {
        Some(_) => McpRunFolder(None),
        None => McpRunFolder(Some(
            new_process_folder(&std::env::temp_dir(), "quarterdeck-agent-")
                .map_err(AgentRunError::Start)?,
        )),
    };
    let working_folder = workspace
        .or(folder.0.as_deref())
        .ok_or(AgentRunError::NoAnswer)?;
    let mut server = McpServer::start(handler.clone())
        .map_err(|error| AgentRunError::Failed(error.to_string()))?;
    let outcome = run_process(
        program,
        prompt,
        working_folder,
        workspace.is_some(),
        format,
        cancel,
        on_progress,
        &server,
        handler.as_ref(),
    );

    // Serialize revocation with authoring, then close sockets and join transport handlers.
    handler.close();
    let shutdown = server
        .shutdown()
        .map_err(|error| AgentRunError::Failed(error.to_string()));
    let answer = outcome?;
    shutdown?;
    if cancel.load(Ordering::Relaxed) {
        return Err(AgentRunError::Cancelled);
    }
    Ok(answer)
}

/// Accepts only the final schema's single, nonempty receipt field.
pub(crate) fn completion_receipt(answer: &AgentAnswer) -> Result<&str, AgentRunError> {
    let object = answer
        .output
        .as_object()
        .filter(|object| object.len() == 1)
        .ok_or_else(|| {
            AgentRunError::Failed("The completion must contain only validation_receipt.".into())
        })?;
    object
        .get("validation_receipt")
        .and_then(Value::as_str)
        .filter(|receipt| !receipt.is_empty())
        .ok_or_else(|| AgentRunError::Failed("The completion has no validation receipt.".into()))
}

/// Rejects inherited modes known to remove explicit MCP tools or subscription login.
fn check_environment() -> Result<(), AgentRunError> {
    for name in ["CLAUDE_CODE_SAFE_MODE", "CLAUDE_CODE_SIMPLE"] {
        if std::env::var(name).is_ok_and(|value| {
            !matches!(
                value.to_ascii_lowercase().as_str(),
                "" | "0" | "false" | "no"
            )
        }) {
            return Err(AgentRunError::Failed(format!(
                "{name} is enabled and conflicts with the review's MCP connection. Disable it before starting this run."
            )));
        }
    }
    Ok(())
}

/// Launches Claude with only the built-in reads and this endpoint's exact tools.
#[allow(clippy::too_many_arguments)]
fn run_process(
    program: &Path,
    prompt: &str,
    working_folder: &Path,
    has_workspace: bool,
    format: McpRunFormat<'_>,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
    server: &McpServer,
    handler: &dyn RunMcpHandler,
) -> Result<AgentAnswer, AgentRunError> {
    let tools = if has_workspace {
        "Read,Grep,Glob,Agent"
    } else {
        ""
    };
    let mut allowed: Vec<String> = format
        .tool_names
        .iter()
        .map(|name| format!("mcp__quarterdeck_run__{name}"))
        .collect();
    if has_workspace {
        allowed.extend(["Read", "Grep", "Glob", "Agent"].map(str::to_owned));
    }
    let mut command = Command::new(program);
    command
        .current_dir(working_folder)
        .args([
            "-p",
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
            "--restricted",
            "--disable-slash-commands",
            "--setting-sources",
            "",
            "--settings",
            ISOLATION_SETTINGS,
            "--strict-mcp-config",
            "--mcp-config",
            &server.config_json(),
            "--permission-mode",
            "dontAsk",
            "--permission-prompts",
            "none",
            "--tools",
            tools,
            "--allowedTools",
            &allowed.join(","),
            "--no-session-persistence",
            "--json-schema",
            MCP_COMPLETION_SCHEMA,
            "--append-system-prompt",
            format.guide,
        ])
        .env("QUARTERDECK_MCP_URL", server.endpoint())
        .env("QUARTERDECK_MCP_TOKEN", server.token())
        .env("ENABLE_TOOL_SEARCH", "false");
    if format.model == McpModel::OpusHigh {
        command.args(["--model", "opus", "--effort", "high"]);
    }
    // A private process group contains descendants that might retain redirected pipes.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        command.process_group(0);
    }
    let mut process = McpChild {
        child: command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(AgentRunError::Start)?,
        reaped: false,
    };

    collect_process(&mut process, prompt, cancel, on_progress, server, handler)
}

/// Collects output without an unbounded join on a descendant's inherited pipe.
fn collect_process(
    process: &mut McpChild,
    prompt: &str,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
    server: &McpServer,
    handler: &dyn RunMcpHandler,
) -> Result<AgentAnswer, AgentRunError> {
    // Threads report completion through channels; the process group owns their pipe lifetime.
    let (prompt_sender, prompt_completion) = mpsc::channel();
    if let Some(mut stdin) = process.child.stdin.take() {
        let prompt = prompt.to_owned();
        std::thread::spawn(move || {
            let _ = prompt_sender.send(stdin.write_all(prompt.as_bytes()));
        });
    }
    let (error_sender, error_completion) = mpsc::channel();
    if let Some(stderr) = process.child.stderr.take() {
        std::thread::spawn(move || {
            let _ = error_sender.send(read_error_tail(stderr));
        });
    }
    let outcome = follow_events(&mut process.child, cancel, on_progress, handler);
    if outcome.is_err() {
        handler.close();
        stop_mcp_child(&mut process.child);
        process.reaped = true;
        let errors = error_completion
            .recv_timeout(CANCEL_INTERVAL)
            .unwrap_or_default();
        return match outcome {
            Err(AgentRunError::NoAnswer) if !errors.trim().is_empty() => {
                Err(AgentRunError::Failed(redact(&errors, server)))
            }
            other => other.map_err(|error| redact_error(error, server)),
        };
    }

    // Require pipe completion before reaping the group leader, so cleanup retains its identity.
    let pipes = wait_pipe(&prompt_completion, cancel).and_then(|written| {
        written.map_err(AgentRunError::Start)?;
        wait_pipe(&error_completion, cancel)
    });
    let errors = match pipes {
        Ok(errors) => redact(&errors, server),
        Err(error) => {
            handler.close();
            return Err(error);
        }
    };
    let status = wait_for_exit(&mut process.child, cancel).inspect_err(|_| handler.close())?;
    process.reaped = true;
    if !status.success() {
        return Err(AgentRunError::Failed(format!(
            "Claude exited with {status}: {}",
            errors.trim()
        )));
    }

    // Known endpoint credentials are removed even if a CLI diagnostic unexpectedly echoes them.
    let mut answer = outcome?;
    answer.transcript = answer
        .transcript
        .into_iter()
        .map(|line| redact(&line, server))
        .collect();
    Ok(answer)
}

/// Waits for one pipe worker with both a deadline and prompt cancellation.
fn wait_pipe<T>(receiver: &mpsc::Receiver<T>, cancel: &AtomicBool) -> Result<T, AgentRunError> {
    let deadline = Instant::now() + PIPE_COMPLETION_GRACE;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(AgentRunError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(AgentRunError::Failed(
                "Claude left an output pipe open after completion.".into(),
            ));
        }
        match receiver.recv_timeout(CANCEL_INTERVAL) {
            Ok(value) => return Ok(value),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(AgentRunError::Failed(
                    "Claude's pipe reader stopped unexpectedly.".into(),
                ));
            }
        }
    }
}

/// Terminates the private process group before reaping its leader to prevent PID reuse.
fn stop_mcp_child(child: &mut Child) {
    #[cfg(unix)]
    {
        use nix::sys::signal::{Signal, killpg};
        use nix::unistd::Pid;
        if let Ok(identifier) = i32::try_from(child.id()) {
            let group = Pid::from_raw(identifier);
            let _ = killpg(group, Signal::SIGTERM);
            std::thread::sleep(STOP_GRACE);
            let _ = killpg(group, Signal::SIGKILL);
            let _ = child.wait();
            return;
        }
    }
    crate::stop_child::stop_child(child);
}

fn redact(text: &str, server: &McpServer) -> String {
    text.replace(server.token(), "[redacted]")
        .replace(server.endpoint(), "[run endpoint]")
}

fn redact_error(error: AgentRunError, server: &McpServer) -> AgentRunError {
    match error {
        AgentRunError::Failed(message) => AgentRunError::Failed(redact(&message, server)),
        other => other,
    }
}

/// Continues through EOF so duplicate or unsuccessful terminal events are rejected.
fn follow_events(
    child: &mut Child,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
    handler: &dyn RunMcpHandler,
) -> Result<AgentAnswer, AgentRunError> {
    let stdout = child.stdout.take().ok_or(AgentRunError::NoAnswer)?;
    let receiver = read_stdout(stdout);
    let mut transcript = Vec::new();
    let mut model = None;
    let mut tool_calls = 0;
    let mut terminal = None;
    let mut terminal_deadline = None;
    loop {
        // Server acceptance and repair progress remains authoritative while Claude is silent.
        for progress in handler.drain_progress() {
            on_progress(progress);
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(AgentRunError::Cancelled);
        }
        if terminal_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(AgentRunError::Failed(
                "Claude did not close its stream after the terminal result.".into(),
            ));
        }
        let line = match receiver.recv_timeout(CANCEL_INTERVAL) {
            Ok(Ok(line)) => line,
            Ok(Err(error)) => {
                return Err(AgentRunError::Failed(format!(
                    "Cannot read Claude output: {error}"
                )));
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        let event = ClaudeEvent::parse(&line);
        if !is_partial_message(&line) {
            transcript.push(line.clone());
        }
        match event {
            ClaudeEvent::Started {
                model: started_model,
            } => {
                model = started_model;
                on_progress(AgentProgress::Started);
            }
            ClaudeEvent::Thinking => on_progress(AgentProgress::Thinking),
            ClaudeEvent::ToolStarted(tool) => {
                if tool != "StructuredOutput" {
                    tool_calls += 1;
                    on_progress(AgentProgress::Exploring { tool_calls });
                }
            }
            ClaudeEvent::Finished(result) => {
                if terminal.is_some() {
                    return Err(AgentRunError::Failed(
                        "Claude returned more than one terminal result.".into(),
                    ));
                }
                let value: Value = serde_json::from_str(&line)
                    .map_err(|error| AgentRunError::Failed(error.to_string()))?;
                if result.is_error || value["subtype"] != "success" {
                    return Err(AgentRunError::Failed(result.message));
                }
                let output = result.structured_output.ok_or(AgentRunError::NoAnswer)?;
                terminal = Some(AgentAnswer {
                    output,
                    cost_usd: result.cost_usd,
                    session_id: result.session_id,
                    model: model.clone(),
                    transcript: Vec::new(),
                });
                terminal_deadline = Some(Instant::now() + TERMINAL_EXIT_GRACE);
            }
            ClaudeEvent::InputChunk(_) | ClaudeEvent::Other => {}
        }
    }
    let mut answer = terminal.ok_or(AgentRunError::NoAnswer)?;
    answer.transcript = transcript;
    Ok(answer)
}

/// Reads at most the permitted stream size, reporting exhaustion rather than fake EOF.
fn read_stdout(
    stdout: impl std::io::Read + Send + 'static,
) -> mpsc::Receiver<std::io::Result<String>> {
    let (sender, receiver) = mpsc::sync_channel(128);
    std::thread::spawn(move || {
        // Read one byte beyond the cap so resource exhaustion cannot look like true EOF.
        let mut reader = std::io::BufReader::new(stdout.take(MAX_STREAM_BYTES + 1));
        let mut consumed = 0_u64;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(count) => {
                    consumed += u64::try_from(count).unwrap_or(u64::MAX);
                    if consumed > MAX_STREAM_BYTES {
                        let _ = sender.send(Err(std::io::Error::other(
                            "Claude output exceeded the 32 MiB run limit.",
                        )));
                        break;
                    }
                    if sender
                        .send(Ok(line.trim_end_matches(['\n', '\r']).to_owned()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(error));
                    break;
                }
            }
        }
    });
    receiver
}

/// Whole stream messages preserve tool results; partial deltas duplicate their contents.
fn is_partial_message(line: &str) -> bool {
    serde_json::from_str::<Value>(line).is_ok_and(|value| value["type"] == "stream_event")
}

/// std has no cancellable child wait; a bounded timed wait checks cancellation.
fn wait_for_exit(child: &mut Child, cancel: &AtomicBool) -> Result<ExitStatus, AgentRunError> {
    use wait_timeout::ChildExt as _;
    let deadline = Instant::now() + TERMINAL_EXIT_GRACE;
    while Instant::now() < deadline {
        if cancel.load(Ordering::Relaxed) {
            return Err(AgentRunError::Cancelled);
        }
        if let Some(status) = child
            .wait_timeout(CANCEL_INTERVAL)
            .map_err(AgentRunError::Start)?
        {
            return Ok(status);
        }
    }
    Err(AgentRunError::Failed(
        "Claude did not exit after its terminal result.".into(),
    ))
}

/// Drains all stderr while retaining only a bounded diagnostic tail.
fn read_error_tail(mut stderr: impl std::io::Read) -> String {
    let mut output = String::new();
    let mut buffer = [0_u8; 4096];
    while let Ok(count) = stderr.read(&mut buffer) {
        if count == 0 {
            break;
        }
        output.push_str(&String::from_utf8_lossy(&buffer[..count]));
        output = tail(&output, MAX_ERROR_OUTPUT_CHARS);
    }
    output
}
