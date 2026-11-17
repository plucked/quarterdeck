#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use qd_studio::{AgentProgress, AgentRunError, GuidedReviewRun, run_guided_review};

/// Real HTTP authoring returns typed buckets only after matching terminal success.
#[test]
fn repairs_via_mcp_and_launches_with_exact_isolation() {
    let folder = tempfile::tempdir().unwrap();
    let program = fake_program(folder.path(), "redaction");
    let paths = vec!["src/lib.rs".into(), "README.md".into()];
    let mut progress = Vec::new();
    let result = run_guided_review(
        &program,
        "Synthetic prompt",
        Some(folder.path()),
        run(&paths),
        &AtomicBool::new(false),
        &mut |event| progress.push(event),
    )
    .unwrap();
    assert_eq!(result.buckets.len(), 1);
    assert_eq!(result.buckets[0].files, paths);
    assert_eq!(result.answer.model.as_deref(), Some("offline-opus"));
    assert_eq!(result.answer.cost_usd, Some(0.0));
    assert!(
        result
            .answer
            .transcript
            .iter()
            .any(|line| line.contains("\"credential\": \"[redacted]\""))
    );
    assert_eq!(result.answer.session_id.as_deref(), Some("offline-session"));
    assert_eq!(
        result.answer.output["validation_receipt"],
        result.validation_receipt
    );
    assert!(
        result
            .audit
            .iter()
            .any(|entry| entry.issue_codes == ["duplicate_file_in_bucket"])
    );
    assert!(
        result
            .audit
            .iter()
            .any(|entry| entry.issue_codes == ["missing_paths"])
    );
    assert!(
        progress
            .iter()
            .any(|event| matches!(event, AgentProgress::ReviewingGuide { issues: 1, .. }))
    );
    assert!(progress.iter().any(|event| matches!(
        event,
        AgentProgress::ReviewingGuide {
            covered_files: 2,
            total_files: 2,
            issues: 0,
            ..
        }
    )));

    // Configuration contains environment placeholders, precise permissions, and no dollar cap.
    let arguments: Vec<String> = serde_json::from_str(
        &std::fs::read_to_string(folder.path().join("arguments.json")).unwrap(),
    )
    .unwrap();
    for flag in [
        "--restricted",
        "--disable-slash-commands",
        "--strict-mcp-config",
        "--no-session-persistence",
    ] {
        assert!(arguments.iter().any(|argument| argument == flag));
    }
    for forbidden in ["--safe-mode", "--bare", "--max-budget-usd"] {
        assert!(!arguments.iter().any(|argument| argument == forbidden));
    }
    assert_eq!(flag_value(&arguments, "--tools"), "Read,Grep,Glob,Agent");
    assert_eq!(flag_value(&arguments, "--setting-sources"), "");
    assert_eq!(flag_value(&arguments, "--model"), "opus");
    assert_eq!(flag_value(&arguments, "--effort"), "high");
    let configuration = flag_value(&arguments, "--mcp-config");
    assert!(configuration.contains("${QUARTERDECK_MCP_TOKEN}"));
    assert!(configuration.contains("${QUARTERDECK_MCP_URL}"));
    assert!(!configuration.contains("127.0.0.1"));
    let permissions = flag_value(&arguments, "--allowedTools");
    assert!(!permissions.contains('*'));
    assert!(permissions.contains("mcp__quarterdeck_run__guide_finish"));
    assert_eq!(
        std::fs::read_to_string(folder.path().join("prompt.txt")).unwrap(),
        "Synthetic prompt"
    );
    assert_endpoint_closed(folder.path());
}

/// A receipt cannot rescue unsuccessful, duplicated, absent, or mismatched completion.
#[test]
fn rejects_unpublished_or_unsuccessful_terminal_states() {
    for mode in [
        "no_finish",
        "wrong_receipt",
        "failed_terminal",
        "failed_exit",
        "duplicate_terminal",
        "output_limit",
        "inherited_stderr",
        "login",
    ] {
        let folder = tempfile::tempdir().unwrap();
        let program = fake_program(folder.path(), mode);
        let paths = vec!["src/lib.rs".into(), "README.md".into()];
        let result = run_guided_review(
            &program,
            "Synthetic",
            Some(folder.path()),
            run(&paths),
            &AtomicBool::new(false),
            &mut |_| {},
        );
        assert!(result.is_err(), "accepted mode {mode}");
        if !matches!(mode, "login" | "no_finish") {
            assert!(
                folder.path().join("finished.txt").exists(),
                "mode {mode} failed before sealing: {result:?}"
            );
        }
        if mode == "login" {
            assert!(result.unwrap_err().to_string().contains("Please log in"));
        }
        assert_endpoint_closed(folder.path());
    }
}

/// Cancelling a silent CLI promptly stops it and revokes its endpoint.
#[test]
fn cancellation_closes_the_run_endpoint() {
    let folder = tempfile::tempdir().unwrap();
    let program = fake_program(folder.path(), "cancel_descendant");
    let paths = vec!["src/lib.rs".into(), "README.md".into()];
    let cancel = Arc::new(AtomicBool::new(false));
    let trigger = Arc::clone(&cancel);
    let endpoint_file = folder.path().join("endpoint.txt");
    let started = Instant::now();
    let timer = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !endpoint_file.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        trigger.store(true, Ordering::Relaxed);
    });
    let result = run_guided_review(
        &program,
        "Synthetic",
        Some(folder.path()),
        run(&paths),
        &cancel,
        &mut |_| {},
    );
    timer.join().unwrap();
    assert!(matches!(result, Err(AgentRunError::Cancelled)));
    assert!(started.elapsed() < Duration::from_secs(7));
    assert_endpoint_closed(folder.path());
}

/// Installs the offline fake CLI in a private working folder.
fn fake_program(folder: &std::path::Path, mode: &str) -> std::path::PathBuf {
    let program = folder.join("fake-claude");
    std::fs::write(&program, include_str!("fixtures/guided_fake_claude.py")).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(folder.join("mode.txt"), mode).unwrap();
    program
}

fn run(paths: &[String]) -> GuidedReviewRun<'_> {
    GuidedReviewRun {
        run_id: "offline-run",
        snapshot_id: "offline-snapshot",
        changed_paths: paths,
    }
}

fn flag_value<'a>(arguments: &'a [String], name: &str) -> &'a str {
    let position = arguments
        .iter()
        .position(|argument| argument == name)
        .unwrap();
    &arguments[position + 1]
}

/// A finished or failed run must leave no listening socket behind.
fn assert_endpoint_closed(folder: &std::path::Path) {
    let endpoint = std::fs::read_to_string(folder.join("endpoint.txt")).unwrap();
    let address = endpoint
        .strip_prefix("http://")
        .unwrap()
        .strip_suffix("/mcp")
        .unwrap();
    assert!(
        std::net::TcpStream::connect_timeout(&address.parse().unwrap(), Duration::from_millis(100))
            .is_err()
    );
}
