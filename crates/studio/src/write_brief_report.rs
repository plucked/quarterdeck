use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::brief_record::BriefRecord;
use super::code_access::CodeAccess;
use super::prepare_workspace::{DIFF_FILE, HEAD_FOLDER};
use super::shown_page::ShownPage;
use super::studio_guide::STUDIO_GUIDE;
use super::studio_schema::STUDIO_SCHEMA;
use super::utc_stamp::utc_stamp;

/// The report's entry point, which names the other files.
pub const REPORT_FILE: &str = "report.md";
/// The end of `report.md`: what each file in the folder holds, and where in
/// Quarterdeck the brief is made.
const FOLDER_GUIDE: &str = "## Files in this folder\n\n\
- `prompt.md`: the user message sent to Claude Code on stdin (the pull request, the reader, where the code is, the numbered diffs anchors cite).\n\
- `system-prompt.md`: the authoring guide appended to the system prompt (`--append-system-prompt`), as compiled into this build.\n\
- `schema.json`: the JSON Schema the answer had to match (`--json-schema`), as compiled into this build.\n\
- `answer.json`: Claude's structured answer, before the app's checks (only for a brief written since the app started; one loaded from storage keeps only the checked brief).\n\
- `rejected.md`: the widgets the app's checks left out, with the reasons.\n\
- `transcript.jsonl`: the run's events without the partial-message stream: the init (model, tools), whole assistant messages with their tool calls, tool results, the result (only for a brief written since the app started).\n\n\
## Where Quarterdeck makes the brief\n\n\
- `crates/studio/src/studio_guide.md`: the authoring guide (system prompt).\n\
- `crates/studio/src/studio_schema.json`: the answer's schema; `crates/studio/src/studio_widget.rs` and siblings: the widget types.\n\
- `crates/studio/src/studio_prompt.rs`: the user message; `reader_brief` sets the onboarding per reader.\n\
- `crates/studio/src/parse_studio.rs`, `wire_graph.rs`, `wire_sequence.rs`, `anchor_targets.rs`: the checks that leave widgets out.\n\
- `crates/studio/src/run_claude.rs`: the Claude Code flags and confinement; `prepare_workspace.rs`: the checkout.\n\
- `crates/app/src/brief_view/`: how the brief is drawn (layouts of diagrams in `graph_layout.rs`, `sequence_layout.rs`).\n\
- `crates/app/src/pull_request_detail/brief_actions.rs`: what is sent and when.\n\
- `docs/research/ai-review-studio.md`: the design and the trials so far.\n";

/// Where brief reports go: `~/Library/Application Support/Quarterdeck/brief-reports`,
/// shared by release and development builds; `None` without such a folder.
pub fn report_root() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("Quarterdeck").join("brief-reports"))
}

/// Writes a report on the brief `record` describes, with the reader's
/// `feedback` and the page they had open (`shown_page`), into a new folder
/// under `root` named after the time, the repository and the pull request,
/// and returns the path of its `report.md`. Besides that entry point the folder holds the prompt, the
/// system prompt and schema as compiled into this build (`app_version`),
/// the raw answer, what the checks left out and the transcript, so a later
/// session can see exactly how the brief came about.
pub fn write_brief_report(
    root: &Path,
    record: &BriefRecord,
    feedback: &str,
    shown_page: &ShownPage,
    app_version: &str,
) -> std::io::Result<PathBuf> {
    // A folder of its own, named so reports sort by time.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let stamp = utc_stamp(now);
    let name = format!(
        "{}-{}-{}",
        stamp.file_name,
        record.repository.replace('/', "-"),
        record.number
    );
    let folder = new_report_folder(root, &name)?;

    // The inputs and outputs of the run, as they were.
    std::fs::write(folder.join("prompt.md"), &record.prompt)?;
    std::fs::write(folder.join("system-prompt.md"), STUDIO_GUIDE)?;
    std::fs::write(folder.join("schema.json"), STUDIO_SCHEMA)?;
    // The raw answer and transcript exist for briefs written this session.
    if let Some(answer) = &record.answer {
        let answer = serde_json::to_string_pretty(answer).unwrap_or_default();
        std::fs::write(folder.join("answer.json"), answer)?;
    }
    if let Some(transcript) = &record.transcript {
        let mut lines = transcript.join("\n");
        lines.push('\n');
        std::fs::write(folder.join("transcript.jsonl"), lines)?;
    }
    std::fs::write(folder.join("rejected.md"), rejected_text(record))?;

    // The entry point.
    let report = folder.join(REPORT_FILE);
    std::fs::write(
        &report,
        report_text(record, feedback, shown_page, app_version, &stamp.readable),
    )?;
    Ok(report)
}

/// A new folder `name` under `root`, or `name-2`, `name-3`… when a report
/// of the same pull request was saved in the same second, so no report
/// overwrites another.
fn new_report_folder(root: &Path, name: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(root)?;
    let mut attempt = 1_u32;
    loop {
        let folder = match attempt {
            1 => root.join(name),
            _ => root.join(format!("{name}-{attempt}")),
        };
        match std::fs::create_dir(&folder) {
            Ok(()) => return Ok(folder),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => attempt += 1,
            Err(error) => return Err(error),
        }
    }
}

/// `report.md`: the reader's words and the page they had open first, then
/// what the brief was about, how it was made, what it read, the files here,
/// and where in Quarterdeck the brief is produced.
fn report_text(
    record: &BriefRecord,
    feedback: &str,
    shown_page: &ShownPage,
    app_version: &str,
    reported: &str,
) -> String {
    let mut text = String::new();
    write_title(&mut text, record, app_version, reported);
    write_feedback(&mut text, feedback, shown_page);
    write_subject(&mut text, record);
    write_run(&mut text, record);
    write_reads(&mut text, record);
    text.push_str(FOLDER_GUIDE);
    text
}

/// The heading, and when and why the report was made.
fn write_title(text: &mut String, record: &BriefRecord, app_version: &str, reported: &str) {
    let _ = writeln!(
        text,
        "# Brief report: {} #{}\n",
        record.repository, record.number
    );
    let _ = writeln!(
        text,
        "Reported {reported} with Quarterdeck {app_version}. A reader found something wrong or weird in a brief Quarterdeck had Claude Code write; this folder holds everything about how that brief was made, so the brief tool (prompt, schema, checks, rendering) can be improved.\n"
    );
}

/// What the reader says, and the page "here" means.
fn write_feedback(text: &mut String, feedback: &str, shown_page: &ShownPage) {
    let _ = writeln!(text, "## What is wrong or weird (the reader's words)\n");
    let _ = writeln!(text, "{}\n", feedback.trim());
    let _ = writeln!(
        text,
        "They had page {} of {}, \"{}\", open when they wrote this, so \"here\" means that page.\n",
        shown_page.number, shown_page.count, shown_page.title
    );
}

/// What the brief was about: the pull request, its head and layers, the
/// reader and the code the run had.
fn write_subject(text: &mut String, record: &BriefRecord) {
    let _ = writeln!(text, "## The brief\n");
    let _ = writeln!(text, "- Pull request: {}", record.url);
    let _ = writeln!(text, "- Subject: {}", record.subject);
    if let Some(head) = &record.head_sha {
        let _ = writeln!(text, "- Written for head commit `{head}`");
    }
    if !record.layers.is_empty() {
        let layers: Vec<String> = record
            .layers
            .iter()
            .map(|layer| format!("#{} {}", layer.number, layer.title))
            .collect();
        let _ = writeln!(text, "- Stack layers, bottom first: {}", layers.join("; "));
    }
    let _ = writeln!(
        text,
        "- Written for a reader who {}",
        record.reader.reader_description()
    );
    match (&record.code, &record.head_sha) {
        (CodeAccess::Workspace, Some(head)) => {
            let _ = writeln!(
                text,
                "- Code: a checkout of head `{head}` (`{HEAD_FOLDER}/`), plus the pull request's diff (`{DIFF_FILE}`)"
            );
        }
        (CodeAccess::Workspace, None) => {
            let _ = writeln!(text, "- Code: a checkout of the head commit");
        }
        (CodeAccess::DiffOnly(reason), _) => {
            let _ = writeln!(text, "- Code: the diffs only ({reason})");
        }
    }
}

/// How the run went: model, time, cost, session, and the answer's shape.
fn write_run(text: &mut String, record: &BriefRecord) {
    let _ = writeln!(
        text,
        "- Run: model {}, {:.0} s, cost {}, session {}",
        record.model.as_deref().unwrap_or("unknown"),
        record.duration.as_secs_f64(),
        record
            .cost_usd
            .map_or_else(|| "unknown".to_owned(), |cost| format!("${cost:.2}")),
        record.session_id.as_deref().unwrap_or("unknown")
    );
    let shape = record
        .answer
        .as_ref()
        .map_or_else(|| "not kept (loaded from storage)".to_owned(), answer_shape);
    let _ = writeln!(
        text,
        "- Answer: {shape}; {} widget{} left out by the app's checks\n",
        record.rejected.len(),
        if record.rejected.len() == 1 { "" } else { "s" }
    );
}

/// What the agent read, in order.
fn write_reads(text: &mut String, record: &BriefRecord) {
    let reads = &record.tool_calls;
    let _ = writeln!(text, "## What the run read ({} tool calls)\n", reads.len());
    if reads.is_empty() {
        let _ = writeln!(text, "Nothing: it wrote from the prompt alone.\n");
    } else {
        for read in reads {
            let _ = writeln!(text, "- {}: {}", read.tool, read.input);
        }
        text.push('\n');
    }
}

/// `rejected.md`: each widget the checks left out and why.
fn rejected_text(record: &BriefRecord) -> String {
    if record.rejected.is_empty() {
        return "Nothing was left out.\n".to_owned();
    }
    let mut text = String::from("# Left out by the app's checks\n\n");
    for rejected in &record.rejected {
        let _ = writeln!(text, "- {}: {}", rejected.widget, rejected.reason);
    }
    text
}

/// The answer's pages and widget counts, such as `3 pages (Overview: 5
/// widgets, The hold: 4 widgets, …)`.
fn answer_shape(answer: &Value) -> String {
    let Some(pages) = answer.get("pages").and_then(Value::as_array) else {
        return "no pages".to_owned();
    };
    let described: Vec<String> = pages
        .iter()
        .map(|page| {
            let title = page.get("title").and_then(Value::as_str).unwrap_or("?");
            let widgets = page
                .get("widgets")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            format!("{title}: {widgets} widgets")
        })
        .collect();
    format!("{} pages ({})", pages.len(), described.join(", "))
}
