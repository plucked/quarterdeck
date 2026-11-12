use std::io::{Read as _, Write as _};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;

use super::agent_answer::AgentAnswer;
use super::agent_progress::AgentProgress;
use super::agent_run_error::AgentRunError;
use super::claude_event::ClaudeEvent;
use super::new_process_folder::new_process_folder;
use super::output_lines::{NextLine, OutputLines};
use super::smart_filter_guide::{SMART_FILTER_GUIDE, SMART_FILTER_SCHEMA};
use super::stop_child::stop_child;
use super::studio_guide::STUDIO_GUIDE;
use super::studio_schema::STUDIO_SCHEMA;
use super::text_tail::tail;

/// The most a run may spend before Claude Code stops it, in US dollars.
const MAX_BUDGET_USD: &str = "5";

/// The tools an agent with a workspace may use: reading, and subagents
/// (`Agent`) to read a large change in parallel. A subagent gets only the
/// reading tools and the same confinement (verified with Claude Code
/// 2.1.318: no Bash, reads outside the folder refused).
const READ_ONLY_TOOLS: &str = "Read,Grep,Glob,Agent";
/// The tool whose input is the structured answer.
const ANSWER_TOOL: &str = "StructuredOutput";

/// The start of an empty run folder's name (a run without a workspace).
const RUN_FOLDER_PREFIX: &str = "quarterdeck-studio-";
/// Characters of the answer between two writing progress reports.
const WRITING_STEP: usize = 100;
/// How much of the CLI's error output is kept for the error message.
const MAX_ERROR_OUTPUT_CHARS: usize = 2_000;

/// Runs Claude Code headless on `prompt` and returns its studio-shaped
/// answer. Blocks until the run ends; call it off the main thread.
///
/// The run is confined in either case: `--restricted` (no tool that runs
/// code, no settings files, file tools kept to the working folder),
/// `--safe-mode` (no CLAUDE.md, hooks, skills or plugins, so a checked-out
/// repository cannot instruct it), no MCP servers, nothing that would ask
/// for permission allowed, and a spending limit. With a `workspace` it
/// starts there with only `Read`, `Grep`, `Glob` and `Agent`; without one it starts
/// in a new empty folder with no tools at all (only the structured-output
/// tool remains). The `Agent` tool starts subagents, which read under the
/// same confinement. The answer must match [`STUDIO_SCHEMA`]; Claude Code
/// re-prompts until it does. Progress is reported per streamed event;
/// setting `cancel` stops the process within a fifth of a second, even
/// while it prints nothing: `SIGTERM`, then `SIGKILL` after a grace period.
pub fn run_claude(
    program: &Path,
    prompt: &str,
    workspace: Option<&Path>,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
) -> Result<AgentAnswer, AgentRunError> {
    run_configured(
        program,
        prompt,
        workspace,
        &RunFormat::STUDIO,
        cancel,
        on_progress,
    )
}

/// Runs one smart-filter generation with Opus at high effort, using only
/// the same confined read tools as briefs and no delegated agents.
pub fn run_smart_filter(
    program: &Path,
    prompt: &str,
    workspace: Option<&Path>,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
) -> Result<AgentAnswer, AgentRunError> {
    run_configured(
        program,
        prompt,
        workspace,
        &RunFormat::SMART_FILTER,
        cancel,
        on_progress,
    )
}

struct RunFormat {
    schema: &'static str,
    guide: &'static str,
    tools: &'static str,
    opus_high: bool,
}

impl RunFormat {
    const STUDIO: Self = Self {
        schema: STUDIO_SCHEMA,
        guide: STUDIO_GUIDE,
        tools: READ_ONLY_TOOLS,
        opus_high: false,
    };
    const SMART_FILTER: Self = Self {
        schema: SMART_FILTER_SCHEMA,
        guide: SMART_FILTER_GUIDE,
        tools: "Read,Grep,Glob",
        opus_high: true,
    };
}

fn run_configured(
    program: &Path,
    prompt: &str,
    workspace: Option<&Path>,
    format: &RunFormat,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
) -> Result<AgentAnswer, AgentRunError> {
    if let Some(workspace) = workspace {
        return run_in(
            program,
            prompt,
            workspace,
            format.tools,
            format,
            cancel,
            on_progress,
        );
    }
    // An empty folder of its own, removed afterwards.
    let working_folder = new_process_folder(&std::env::temp_dir(), RUN_FOLDER_PREFIX)
        .map_err(AgentRunError::Start)?;
    let outcome = run_in(
        program,
        prompt,
        &working_folder,
        "",
        format,
        cancel,
        on_progress,
    );
    let _ = std::fs::remove_dir_all(&working_folder);
    outcome
}

/// The run itself, in `working_folder`, with `tools` (comma-separated;
/// empty for none) allowed without asking.
fn run_in(
    program: &Path,
    prompt: &str,
    working_folder: &Path,
    tools: &str,
    format: &RunFormat,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
) -> Result<AgentAnswer, AgentRunError> {
    // Start the CLI with the studio's schema and guide.
    let mut command = Command::new(program);
    command.current_dir(working_folder).args([
        "-p",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--restricted",
        "--safe-mode",
        "--strict-mcp-config",
        "--permission-mode",
        "dontAsk",
        "--permission-prompts",
        "none",
        "--tools",
        tools,
        "--max-budget-usd",
        MAX_BUDGET_USD,
        "--json-schema",
        format.schema,
        "--append-system-prompt",
        format.guide,
    ]);
    if format.opus_high {
        command.args(["--model", "opus", "--effort", "high"]);
    }
    if !tools.is_empty() {
        command.args(["--allowedTools", tools]);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(AgentRunError::Start)?;

    // Feed the prompt and collect error output on threads of their own, so
    // neither pipe can fill up and stall the process.
    let prompt_writer = child.stdin.take().map(|mut stdin| {
        let prompt = prompt.to_owned();
        std::thread::spawn(move || {
            let _ = stdin.write_all(prompt.as_bytes());
        })
    });
    let error_reader = child.stderr.take().map(|mut stderr| {
        std::thread::spawn(move || {
            let mut output = String::new();
            let _ = stderr.read_to_string(&mut output);
            output
        })
    });

    // Follow the event stream until the result line or the end of output.
    let outcome = follow_events(&mut child, cancel, on_progress);
    if let Some(writer) = prompt_writer {
        let _ = writer.join();
    }
    let error_output = error_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    let _ = child.wait();

    // A run that ended without a result says what the CLI printed.
    match outcome {
        Err(AgentRunError::NoAnswer) if !error_output.trim().is_empty() => Err(
            AgentRunError::Failed(tail(error_output.trim(), MAX_ERROR_OUTPUT_CHARS)),
        ),
        other => other,
    }
}

/// Reads the CLI's events, reporting progress, until its result.
fn follow_events(
    child: &mut Child,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
) -> Result<AgentAnswer, AgentRunError> {
    let Some(stdout) = child.stdout.take() else {
        return Err(AgentRunError::NoAnswer);
    };
    let mut written_characters = 0;
    let mut transcript = Vec::new();
    let mut model = None;
    let mut tool_calls = 0;
    let mut is_writing_answer = false;
    let lines = OutputLines::read(stdout);
    loop {
        // Stop the process as soon as the run is no longer wanted, also
        // while it prints nothing.
        let line = match lines.next(cancel) {
            NextLine::Line(line) => line,
            NextLine::Ended => break,
            NextLine::Cancelled => {
                stop_child(child);
                return Err(AgentRunError::Cancelled);
            }
        };
        let event = ClaudeEvent::parse(&line);
        // Whole messages go into the transcript; the partial-message stream
        // only repeats them in pieces.
        if !is_partial_message(&line) {
            transcript.push(line);
        }
        match event {
            ClaudeEvent::Started {
                model: started_with,
            } => {
                model = started_with;
                on_progress(AgentProgress::Started);
            }
            ClaudeEvent::Thinking => on_progress(AgentProgress::Thinking),
            // Tool calls other than the answer are the agent reading code.
            ClaudeEvent::ToolStarted(tool) => {
                is_writing_answer = tool == ANSWER_TOOL;
                if !is_writing_answer {
                    tool_calls += 1;
                    on_progress(AgentProgress::Exploring { tool_calls });
                }
            }
            // The answer's length, reported in steps the reader can see
            // (the app shows tenths of thousands), not per streamed piece.
            ClaudeEvent::InputChunk(characters) if is_writing_answer => {
                let before = written_characters / WRITING_STEP;
                written_characters += characters;
                if written_characters / WRITING_STEP != before {
                    on_progress(AgentProgress::Writing {
                        characters: written_characters,
                    });
                }
            }
            ClaudeEvent::Finished(result) => {
                return match (result.is_error, result.structured_output) {
                    (false, Some(output)) => Ok(AgentAnswer {
                        output,
                        cost_usd: result.cost_usd,
                        session_id: result.session_id,
                        model,
                        transcript,
                    }),
                    (true, _) => Err(AgentRunError::Failed(result.message)),
                    (false, None) => Err(AgentRunError::NoAnswer),
                };
            }
            // Input of a reading tool, and everything else, shows nothing.
            ClaudeEvent::InputChunk(_) | ClaudeEvent::Other => {}
        }
    }
    Err(AgentRunError::NoAnswer)
}

/// Whether `line` is a piece of the partial-message stream
/// (`--include-partial-messages`), which the whole messages repeat.
fn is_partial_message(line: &str) -> bool {
    line.starts_with(r#"{"type":"stream_event""#)
}
