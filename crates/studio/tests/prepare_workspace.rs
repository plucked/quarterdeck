use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;

use qd_studio::{
    DIFF_FILE, HEAD_FOLDER, WorkspaceProgress, WorkspaceSource, clean_orphan_workspaces,
    prepare_workspace,
};

/// Runs git in `folder`, returning its trimmed output.
fn git(folder: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(folder)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {arguments:?}: {output:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// A workspace holds the head commit's files without `.git` (not the
/// earlier commit's), and the diff it was given; dropping it removes the
/// folder.
#[test]
fn checks_out_the_head_and_cleans_up() {
    // A repository with two commits, serving any commit it has.
    let scratch =
        std::env::temp_dir().join(format!("qd-studio-workspace-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let repository = scratch.join("repository");
    std::fs::create_dir_all(&repository).expect("folder");
    git(&repository, &["init", "--quiet"]);
    git(
        &repository,
        &["config", "uploadpack.allowAnySHA1InWant", "true"],
    );
    std::fs::write(repository.join("lib.rs"), "fn wait() -> u64 { 0 }\n").expect("write");
    git(&repository, &["add", "."]);
    git(&repository, &["commit", "--quiet", "-m", "base"]);
    std::fs::write(repository.join("lib.rs"), "fn wait() -> u64 { 600 }\n").expect("write");
    git(&repository, &["commit", "--quiet", "-am", "head"]);
    let head_sha = git(&repository, &["rev-parse", "HEAD"]);

    // Prepare the workspace from it.
    let root = scratch.join("workspaces");
    let source = WorkspaceSource {
        remote_url: repository.display().to_string(),
        head_sha,
        token: None,
    };
    let change = "--- a/lib.rs\n+++ b/lib.rs\n@@ -1 +1 @@\n-fn wait() -> u64 { 0 }\n+fn wait() -> u64 { 600 }\n";
    let mut steps = Vec::new();
    let workspace = prepare_workspace(
        &root,
        &source,
        change,
        &AtomicBool::new(false),
        &mut |step| {
            steps.push(step);
        },
    )
    .expect("the workspace is prepared");

    // The head's tree and the diff are there, and nothing else.
    let path = workspace.path().to_owned();
    let read = |relative: &str| std::fs::read_to_string(path.join(relative)).expect("file exists");
    assert_eq!(
        read(&format!("{HEAD_FOLDER}/lib.rs")),
        "fn wait() -> u64 { 600 }\n"
    );
    assert_eq!(read(DIFF_FILE), change);
    let mut entries: Vec<String> = std::fs::read_dir(&path)
        .expect("folder")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(entries, [DIFF_FILE, HEAD_FOLDER]);
    assert_eq!(
        steps.first(),
        Some(&WorkspaceProgress::Fetching { percent: None })
    );
    assert_eq!(steps.last(), Some(&WorkspaceProgress::Unpacking));

    // A running app's workspace survives the orphan sweep; dropping removes it.
    assert_eq!(clean_orphan_workspaces(&root), 0);
    assert!(path.exists());
    drop(workspace);
    assert!(!path.exists());
    let _ = std::fs::remove_dir_all(&scratch);
}

/// A workspace whose process is gone is removed by the sweep; folders that
/// are not workspaces are left alone.
#[test]
fn sweeps_orphans_only() {
    let root = std::env::temp_dir().join(format!("qd-studio-orphan-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    // Process ids this high are not handed out on macOS (the limit is 99999).
    let orphan = root.join("workspace-999999-1");
    let other = root.join("notes");
    std::fs::create_dir_all(&orphan).expect("folder");
    std::fs::create_dir_all(&other).expect("folder");
    assert_eq!(clean_orphan_workspaces(&root), 1);
    assert!(!orphan.exists());
    assert!(other.exists());
    let _ = std::fs::remove_dir_all(&root);
}
