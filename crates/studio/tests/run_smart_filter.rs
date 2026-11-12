#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use qd_studio::{AgentProgress, AgentRunError, run_smart_filter};

/// Exercise the real process/stream boundary without invoking a paid agent.
#[test]
fn passes_explicit_model_effort_and_schema_and_handles_failure_and_cancel() {
    let folder = std::env::temp_dir().join(format!("qd-smart-run-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let program = folder.join("claude-test");
    let script = r#"#!/bin/sh
printf '%s\n' "$@" > arguments.txt
cat > prompt.txt
printf '%s\n' '{"type":"system","subtype":"init","model":"test-opus"}'
printf '%s\n' '{"type":"result","is_error":false,"structured_output":{"buckets":[]}}'
"#;
    std::fs::write(&program, script).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut progress = Vec::new();
    let answer = run_smart_filter(
        &program,
        "Review all changed files",
        Some(&folder),
        &AtomicBool::new(false),
        &mut |step| progress.push(step),
    )
    .unwrap();
    assert_eq!(answer.model.as_deref(), Some("test-opus"));
    assert_eq!(progress, [AgentProgress::Started]);
    assert_eq!(
        std::fs::read_to_string(folder.join("prompt.txt")).unwrap(),
        "Review all changed files"
    );
    let args = std::fs::read_to_string(folder.join("arguments.txt")).unwrap();
    for flags in [
        "--model\nopus",
        "--effort\nhigh",
        "--tools\nRead,Grep,Glob",
        "--restricted",
        "--safe-mode",
        "--strict-mcp-config",
        "--permission-mode\ndontAsk",
    ] {
        assert!(args.contains(flags), "missing {flags}");
    }
    let schema = args
        .split("--json-schema\n")
        .nth(1)
        .unwrap()
        .split("\n--append-system-prompt")
        .next()
        .unwrap();
    let schema: serde_json::Value = serde_json::from_str(schema).unwrap();
    assert!(schema["properties"]["buckets"].get("maxItems").is_none());
    assert!(schema["properties"]["buckets"]["items"]["properties"]["suggest_viewed"].is_object());

    std::fs::write(
        &program,
        "#!/bin/sh\ncat >/dev/null\necho 'Please log in' >&2\nexit 1\n",
    )
    .unwrap();
    let error = run_smart_filter(
        &program,
        "x",
        Some(&folder),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap_err();
    assert!(error.to_string().contains("Please log in"));

    std::fs::write(&program, "#!/bin/sh\nexec sleep 30\n").unwrap();
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let trigger = cancel.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        trigger.store(true, Ordering::Relaxed);
    });
    let started = std::time::Instant::now();
    let error = run_smart_filter(&program, "x", Some(&folder), &cancel, &mut |_| {}).unwrap_err();
    thread.join().unwrap();
    assert!(matches!(error, AgentRunError::Cancelled));
    assert!(started.elapsed() < Duration::from_secs(5));
    std::fs::remove_dir_all(folder).unwrap();
}
