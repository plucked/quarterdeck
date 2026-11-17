#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::sync::atomic::AtomicBool;

use qd_diff::{DiffOptions, FileDiff};
use qd_studio::{AgentProgress, AnchorTargets, BriefRun, run_brief_review};

/// Semantic page errors repair in one CLI run and return the full accepted report artifact.
#[test]
fn repairs_brief_anchors_and_preserves_the_accepted_artifact() {
    let folder = tempfile::tempdir().unwrap();
    let program = fake_program(folder.path(), "success");
    let mut progress = Vec::new();
    let answer = run_brief_review(
        &program,
        "Synthetic brief",
        Some(folder.path()),
        run(),
        &AtomicBool::new(false),
        &mut |event| progress.push(event),
    )
    .unwrap();
    assert_eq!(answer.studio.pages.len(), 1);
    assert_eq!(answer.studio.pages[0].widgets.len(), 2);
    assert_eq!(
        answer.artifact["pages"][0]["widgets"][1]["anchor"]["start_line"],
        2
    );
    assert_eq!(answer.artifact["lead"], answer.studio.lead);
    assert!(answer.answer.output.get("pages").is_none());
    assert_eq!(
        answer.answer.output["validation_receipt"],
        answer.validation_receipt
    );
    assert_eq!(answer.answer.model.as_deref(), Some("offline-opus"));
    assert_eq!(answer.answer.cost_usd, Some(0.0));
    assert_eq!(
        answer
            .audit
            .iter()
            .filter(|entry| entry.status == "rejected")
            .count(),
        2
    );
    assert!(
        progress
            .iter()
            .any(|event| matches!(event, AgentProgress::ReviewingBrief { issues: 1, .. }))
    );
    let arguments: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(folder.path().join("arguments.json")).unwrap(),
    )
    .unwrap();
    assert!(!arguments.iter().any(|argument| matches!(
        argument.as_str(),
        "--model" | "--effort" | "--safe-mode" | "--bare" | "--max-budget-usd"
    )));
    assert!(
        arguments
            .iter()
            .any(|argument| argument.contains("mcp__quarterdeck_run__brief_finish"))
    );
    assert_endpoint_closed(folder.path());
}

/// A sealed brief remains unpublished when its receipt or terminal completion is invalid.
#[test]
fn rejects_forged_or_failed_brief_completions() {
    for mode in [
        "no_finish",
        "wrong_receipt",
        "failed_terminal",
        "failed_exit",
    ] {
        let folder = tempfile::tempdir().unwrap();
        let program = fake_program(folder.path(), mode);
        let result = run_brief_review(
            &program,
            "Synthetic brief",
            Some(folder.path()),
            run(),
            &AtomicBool::new(false),
            &mut |_| {},
        );
        assert!(result.is_err(), "accepted {mode}");
        if mode != "no_finish" {
            assert!(
                folder.path().join("finished.txt").exists(),
                "failed before sealing: {result:?}"
            );
        }
        assert_endpoint_closed(folder.path());
    }
}

fn run() -> BriefRun<'static> {
    let mut targets = AnchorTargets::default();
    targets.add_file("src/lib.rs", None, &FileDiff::build("@@ -1,3 +1,3 @@\n pub fn greeting(name: &str) -> String {\n-    format!(\"Hello, world!\")\n+    format!(\"Hello, {name}!\")\n }", DiffOptions::default()));
    BriefRun {
        run_id: "offline-brief",
        snapshot_id: "offline-brief-snapshot",
        targets,
    }
}

fn fake_program(folder: &std::path::Path, mode: &str) -> std::path::PathBuf {
    let program = folder.join("fake-claude");
    std::fs::write(&program, include_str!("fixtures/brief_fake_claude.py")).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(folder.join("mode.txt"), mode).unwrap();
    program
}

fn assert_endpoint_closed(folder: &std::path::Path) {
    let endpoint = std::fs::read_to_string(folder.join("endpoint.txt")).unwrap();
    let address = endpoint
        .strip_prefix("http://")
        .unwrap()
        .strip_suffix("/mcp")
        .unwrap();
    assert!(
        std::net::TcpStream::connect_timeout(
            &address.parse().unwrap(),
            std::time::Duration::from_millis(100)
        )
        .is_err()
    );
}
