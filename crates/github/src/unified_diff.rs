use crate::PullRequestFile;

/// The line that starts each file's section in a `git diff`.
const FILE_HEADER: &str = "diff --git ";
/// Where a section's hunks begin; everything from here on is the patch.
const HUNK_HEADER: &str = "@@";
/// The path `git` writes for the missing side of an added or deleted file.
const NO_FILE: &str = "/dev/null";

/// Splits a whole `git diff` (GitHub's `application/vnd.github.diff`
/// answer) into one changed file per section, shaped like the files API's
/// entries: the patch is the section from its first hunk header, as the
/// files API gives it, and absent for binary files and pure renames.
pub(crate) fn files_of_unified_diff(diff: &str) -> Vec<PullRequestFile> {
    // Cut the text at every file header, then read each section on its own.
    let mut sections: Vec<Vec<&str>> = Vec::new();
    for line in diff.lines() {
        match (line.starts_with(FILE_HEADER), sections.last_mut()) {
            (true, _) => sections.push(vec![line]),
            (false, Some(section)) => section.push(line),
            // Text before the first header (none in git's output) belongs to no file.
            (false, None) => {}
        }
    }
    sections
        .iter()
        .filter_map(|section| file_of_section(section))
        .collect()
}

/// The file one section describes; `None` when no path can be read.
fn file_of_section(section: &[&str]) -> Option<PullRequestFile> {
    let first_hunk = section
        .iter()
        .position(|line| line.starts_with(HUNK_HEADER))
        .unwrap_or(section.len());
    let (headers, hunks) = section.split_at(first_hunk);

    // Read the extended headers: they say how the file changed and its paths.
    let mut header = SectionHeader::default();
    for line in headers {
        header.read(line);
    }

    // Count the changed lines of the hunks; header lines start with "@@".
    let additions = hunks.iter().filter(|line| line.starts_with('+')).count();
    let deletions = hunks.iter().filter(|line| line.starts_with('-')).count();
    let (filename, previous_filename) = header.paths(section[0])?;
    Some(PullRequestFile {
        status: header.kind.status().to_owned(),
        previous_filename: (header.kind == ChangeKind::Renamed)
            .then_some(previous_filename)
            .flatten(),
        filename,
        additions: u64::try_from(additions).ok()?,
        deletions: u64::try_from(deletions).ok()?,
        patch: (!hunks.is_empty()).then(|| hunks.join("\n")),
    })
}

/// How a section's file changed, from its extended headers.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum ChangeKind {
    #[default]
    Modified,
    Added,
    Removed,
    Renamed,
    Copied,
}

impl ChangeKind {
    /// The files API's status word for this change.
    fn status(self) -> &'static str {
        match self {
            Self::Modified => "modified",
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Renamed => "renamed",
            Self::Copied => "copied",
        }
    }
}

/// What a section's lines before its first hunk say.
#[derive(Default)]
struct SectionHeader {
    kind: ChangeKind,
    /// From `rename from` / `copy from` or `--- a/…`.
    old_path: Option<String>,
    /// From `rename to` / `copy to` or `+++ b/…`.
    new_path: Option<String>,
}

impl SectionHeader {
    /// Takes in one header line; lines it does not know (`index`, modes,
    /// `similarity index`, `Binary files …`) change nothing.
    fn read(&mut self, line: &str) {
        if line.starts_with("new file mode") {
            self.kind = ChangeKind::Added;
        } else if line.starts_with("deleted file mode") {
            self.kind = ChangeKind::Removed;
        } else if let Some(path) = line.strip_prefix("rename from ") {
            self.kind = ChangeKind::Renamed;
            self.old_path = Some(unquoted(path));
        } else if let Some(path) = line.strip_prefix("rename to ") {
            self.kind = ChangeKind::Renamed;
            self.new_path = Some(unquoted(path));
        } else if let Some(path) = line.strip_prefix("copy from ") {
            self.kind = ChangeKind::Copied;
            self.old_path = Some(unquoted(path));
        } else if let Some(path) = line.strip_prefix("copy to ") {
            self.kind = ChangeKind::Copied;
            self.new_path = Some(unquoted(path));
        } else if let Some(path) = line.strip_prefix("--- ") {
            self.old_path = self.old_path.take().or_else(|| side_path(path, "a/"));
        } else if let Some(path) = line.strip_prefix("+++ ") {
            self.new_path = self.new_path.take().or_else(|| side_path(path, "b/"));
        }
    }

    /// The file's path (the old one for a deleted file) and the path before;
    /// read from the `diff --git a/… b/…` line when no other line names them.
    fn paths(&self, file_header: &str) -> Option<(String, Option<String>)> {
        let (header_old, header_new) = header_line_paths(file_header)?;
        let old_path = self.old_path.clone().unwrap_or(header_old);
        let new_path = self.new_path.clone().unwrap_or(header_new);
        let filename = match self.kind {
            ChangeKind::Removed => old_path.clone(),
            ChangeKind::Modified | ChangeKind::Added | ChangeKind::Renamed | ChangeKind::Copied => {
                new_path
            }
        };
        Some((filename, Some(old_path)))
    }
}

/// The path of a `---`/`+++` line without its `a/`/`b/` prefix; `None` for
/// `/dev/null` (the side that does not exist).
fn side_path(path: &str, prefix: &str) -> Option<String> {
    let path = unquoted(path);
    if path == NO_FILE {
        return None;
    }
    Some(path.strip_prefix(prefix).unwrap_or(&path).to_owned())
}

/// The two paths of `diff --git a/OLD b/NEW`. Paths with spaces are
/// ambiguous here, so when the halves differ the split that gives two equal
/// paths wins (the common unchanged-path case), else the first ` b/`.
fn header_line_paths(line: &str) -> Option<(String, String)> {
    let paths = line.strip_prefix(FILE_HEADER)?;
    if let Some((old, new)) = quoted_pair(paths) {
        return Some((old, new));
    }
    let rest = paths.strip_prefix("a/")?;
    let half = rest.len().saturating_sub(2) / 2;
    if rest.get(half..half + 3) == Some(" b/") && rest[..half] == rest[half + 3..] {
        return Some((rest[..half].to_owned(), rest[half + 3..].to_owned()));
    }
    let (old, new) = rest.split_once(" b/")?;
    Some((old.to_owned(), new.to_owned()))
}

/// The paths of a header whose paths git quoted (`"a/x y" "b/x y"`).
fn quoted_pair(paths: &str) -> Option<(String, String)> {
    let rest = paths.strip_prefix("\"a/")?;
    let (old, rest) = rest.split_once('"')?;
    let new = rest.trim_start().strip_prefix("\"b/")?.strip_suffix('"')?;
    Some((old.to_owned(), new.to_owned()))
}

/// A path without the quotes git puts around unusual names; octal escapes
/// inside stay as written.
fn unquoted(path: &str) -> String {
    path.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "diff --git a/src/main.rs b/src/main.rs
index 1111111..2222222 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,2 +1,2 @@
 fn main() {
-    old();
+    new();
diff --git a/docs/new file.md b/docs/new file.md
new file mode 100644
index 0000000..3333333
--- /dev/null
+++ b/docs/new file.md
@@ -0,0 +1 @@
+hello
diff --git a/gone.txt b/gone.txt
deleted file mode 100644
index 4444444..0000000
--- a/gone.txt
+++ /dev/null
@@ -1 +0,0 @@
-bye
diff --git a/old/name.rs b/new/name.rs
similarity index 100%
rename from old/name.rs
rename to new/name.rs
diff --git a/logo.png b/logo.png
index 5555555..6666666 100644
Binary files a/logo.png and b/logo.png differ
";

    /// Each section becomes a file with the files API's status, paths, counts and patch.
    #[test]
    fn splits_sections_into_files() {
        let files = files_of_unified_diff(DIFF);
        let summary: Vec<(&str, &str, u64, u64, bool)> = files
            .iter()
            .map(|file| {
                (
                    file.filename.as_str(),
                    file.status.as_str(),
                    file.additions,
                    file.deletions,
                    file.patch.is_some(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("src/main.rs", "modified", 1, 1, true),
                ("docs/new file.md", "added", 1, 0, true),
                ("gone.txt", "removed", 0, 1, true),
                ("new/name.rs", "renamed", 0, 0, false),
                ("logo.png", "modified", 0, 0, false),
            ]
        );
        assert_eq!(files[3].previous_filename.as_deref(), Some("old/name.rs"));
        assert_eq!(files[0].previous_filename, None);
    }

    /// The patch starts at the first hunk header, like the files API's.
    #[test]
    fn patch_starts_at_first_hunk() {
        let files = files_of_unified_diff(DIFF);
        assert_eq!(
            files[0].patch.as_deref(),
            Some("@@ -1,2 +1,2 @@\n fn main() {\n-    old();\n+    new();")
        );
    }
}
