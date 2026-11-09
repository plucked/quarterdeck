use std::time::Duration;

use qd_studio::{
    BriefRecord, CodeAccess, REPORT_FILE, ReaderFamiliarity, RejectedWidget, ToolCall,
    write_brief_report,
};
use serde_json::json;

/// A report folder holds the entry point with the reader's words, what the
/// brief was about and what it read, and every input and output of the run.
#[test]
fn writes_every_part_of_a_report() {
    let root = std::env::temp_dir().join(format!("qd-studio-report-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let record = BriefRecord {
        repository: "plucked/quarterdeck".into(),
        number: 42,
        url: "https://github.com/plucked/quarterdeck/pull/42".into(),
        subject: "Pull request #42: Hold refreshes".into(),
        layers: Vec::new(),
        head_sha: Some("bbb222".into()),
        reader: ReaderFamiliarity::Newcomer,
        code: CodeAccess::Workspace,
        prompt: "# Pull request\n…".into(),
        answer: Some(json!({ "lead": "L", "pages": [{ "title": "Overview", "summary": "S", "widgets": [] }] })),
        transcript: Some(vec![
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"head/src/lib.rs"}}]}}"#.into(),
        ]),
        tool_calls: vec![ToolCall {
            tool: "Read".into(),
            input: "file_path: head/src/lib.rs".into(),
        }],
        rejected: vec![RejectedWidget {
            widget: "Overview: graph “Owners”".into(),
            reason: "an edge names an unknown node “x”".into(),
        }],
        cost_usd: Some(0.42),
        session_id: Some("session-1".into()),
        model: Some("claude-opus-5-5".into()),
        duration: Duration::from_secs(65),
        written_at: 1_794_148_320,
    };

    let report = write_brief_report(&root, &record, "The lead is too long.", "0.3.0")
        .expect("the report is written");

    // The entry point names the reader's words, the subject, the reads.
    assert_eq!(
        report.file_name().and_then(|name| name.to_str()),
        Some(REPORT_FILE)
    );
    let text = std::fs::read_to_string(&report).expect("report.md");
    assert!(text.contains("The lead is too long."), "{text}");
    assert!(
        text.contains("head `bbb222`") && text.contains("new to this area"),
        "{text}"
    );
    assert!(
        text.contains("- Read: file_path: head/src/lib.rs"),
        "{text}"
    );
    assert!(text.contains("1 pages (Overview: 0 widgets)"), "{text}");

    // Every other file is there.
    let folder = report.parent().expect("a folder");
    for file in [
        "prompt.md",
        "system-prompt.md",
        "schema.json",
        "answer.json",
        "rejected.md",
        "transcript.jsonl",
    ] {
        assert!(folder.join(file).exists(), "{file} is missing");
    }
    let rejected = std::fs::read_to_string(folder.join("rejected.md")).expect("rejected.md");
    assert!(rejected.contains("unknown node"));

    // A second report of the same pull request in the same second gets a
    // folder of its own, so the first stays.
    let again = write_brief_report(&root, &record, "Second thoughts.", "0.3.0")
        .expect("the second report is written");
    assert_ne!(again.parent(), report.parent());
    let first = std::fs::read_to_string(&report).expect("the first report.md");
    assert!(first.contains("The lead is too long."), "{first}");
    let _ = std::fs::remove_dir_all(&root);
}
