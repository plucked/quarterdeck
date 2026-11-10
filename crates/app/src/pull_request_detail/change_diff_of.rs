use std::fmt::Write as _;

use super::loaded_file::LoadedFile;

/// The loaded files' patches as one unified diff, for the workspace's
/// `change.diff`: every file (generated and moved ones too), with `---` /
/// `+++` headers (`/dev/null` for an added or removed side, the previous
/// path for a rename) so the agent can see what each file was before; a
/// file without a patch is named with why.
pub(super) fn change_diff_of(files: &[LoadedFile]) -> String {
    let mut diff = String::new();
    for loaded in files {
        let file = &loaded.file;
        let old_path = file.previous_filename.as_deref().unwrap_or(&file.filename);
        let (old_side, new_side) = match file.status.as_str() {
            "added" => ("/dev/null".to_owned(), format!("b/{}", file.filename)),
            "removed" => (format!("a/{old_path}"), "/dev/null".to_owned()),
            _ => (format!("a/{old_path}"), format!("b/{}", file.filename)),
        };
        let _ = writeln!(diff, "diff --git a/{old_path} b/{}", file.filename);
        match &file.patch {
            Some(patch) => {
                let _ = writeln!(diff, "--- {old_side}\n+++ {new_side}");
                diff.push_str(patch);
                if !patch.ends_with('\n') {
                    diff.push('\n');
                }
            }
            None => {
                let _ = writeln!(diff, "(no patch: binary, or too large for GitHub to show)");
            }
        }
    }
    diff
}

#[cfg(test)]
mod tests {
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;

    use super::*;

    fn loaded(
        filename: &str,
        previous: Option<&str>,
        status: &str,
        patch: Option<&str>,
    ) -> LoadedFile {
        LoadedFile::build(
            PullRequestFile {
                filename: filename.into(),
                previous_filename: previous.map(Into::into),
                status: status.into(),
                additions: 1,
                deletions: 1,
                patch: patch.map(Into::into),
                sha: None,
            },
            DiffOptions::default(),
        )
    }

    /// The workspace's diff holds every file with headers that say what it
    /// was before: added from nothing, renamed from its old path, and a
    /// file without a patch named.
    #[test]
    fn writes_every_file_into_the_change_diff() {
        let diff = change_diff_of(&[
            loaded(
                "src/new.rs",
                None,
                "added",
                Some("@@ -0,0 +1 @@\n+fn new() {}"),
            ),
            loaded(
                "src/lib.rs",
                Some("src/old.rs"),
                "renamed",
                Some("@@ -1 +1 @@\n-a\n+b\n"),
            ),
            loaded("logo.png", None, "modified", None),
        ]);
        assert!(
            diff.contains("--- /dev/null\n+++ b/src/new.rs\n@@ -0,0 +1 @@\n+fn new() {}\n"),
            "{diff}"
        );
        assert!(
            diff.contains(
                "diff --git a/src/old.rs b/src/lib.rs\n--- a/src/old.rs\n+++ b/src/lib.rs\n"
            ),
            "{diff}"
        );
        assert!(
            diff.contains("diff --git a/logo.png b/logo.png\n(no patch"),
            "{diff}"
        );
    }
}
