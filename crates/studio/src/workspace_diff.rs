use std::path::{Component, Path, PathBuf};

/// The folder of a workspace holding every changed file's numbered diff.
pub(crate) const DIFFS_FOLDER: &str = "diffs";
/// What follows a changed file's path in its diff file's name.
const DIFF_EXTENSION: &str = ".diff";

/// One changed file's diff in the prompt's numbered form, written into the
/// workspace as `diffs/<path>.diff` so the agent can read (and cite) diffs
/// the prompt had no room for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceDiff {
    /// The changed file's path in the repository.
    pub path: String,
    /// The numbered diff: `base head mark text` per line.
    pub listing: String,
}

impl WorkspaceDiff {
    /// Where the diff of `path` is read in the workspace: `diffs/<path>.diff`.
    pub fn relative_path(path: &str) -> String {
        format!("{DIFFS_FOLDER}/{path}{DIFF_EXTENSION}")
    }

    /// Writes every diff under `workspace/diffs/`. A path that would leave
    /// that folder (`..`, absolute) is skipped: paths come from GitHub.
    pub fn write_all(workspace: &Path, diffs: &[Self]) -> std::io::Result<()> {
        let root = workspace.join(DIFFS_FOLDER);
        for diff in diffs {
            let Some(file) = contained_path(&root, &format!("{}{DIFF_EXTENSION}", diff.path))
            else {
                continue;
            };
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(file, &diff.listing)?;
        }
        Ok(())
    }
}

/// `relative` under `root`, if it names a path inside it (only plain
/// components).
fn contained_path(root: &Path, relative: &str) -> Option<PathBuf> {
    let relative = Path::new(relative);
    relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
        .then(|| root.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Diffs land under `diffs/` by path; a path leaving the folder is skipped.
    #[test]
    fn writes_diffs_inside_the_folder_only() {
        let workspace =
            std::env::temp_dir().join(format!("qd-studio-diffs-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&workspace);
        let diff = |path: &str| WorkspaceDiff {
            path: path.into(),
            listing: format!("listing of {path}\n"),
        };
        WorkspaceDiff::write_all(&workspace, &[diff("src/lib.rs"), diff("../escape.rs")])
            .expect("written");
        assert_eq!(
            std::fs::read_to_string(workspace.join("diffs/src/lib.rs.diff")).expect("file"),
            "listing of src/lib.rs\n"
        );
        assert!(!workspace.join("escape.rs.diff").exists());
        let _ = std::fs::remove_dir_all(&workspace);
    }
}
