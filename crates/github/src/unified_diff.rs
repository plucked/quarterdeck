use crate::PullRequestFile;

/// The line that starts each file's section in a `git diff`.
const FILE_HEADER: &str = "diff --git ";
/// Where a section's hunks begin; everything from here on is the patch.
const HUNK_HEADER: &str = "@@";
/// The path `git` writes for the missing side of an added or deleted file.
const NO_FILE: &str = "/dev/null";
/// The extended header naming the blobs before and after: `index OLD..NEW [mode]`.
const INDEX_HEADER: &str = "index ";

/// Splits a whole `git diff` (GitHub's `application/vnd.github.diff`
/// answer) into one changed file per section, shaped like the files API's
/// entries: the patch is the section from its first hunk header, as the
/// files API gives it, and absent for binary files and pure renames.
pub(crate) fn files_of_unified_diff(diff: &str) -> Option<Vec<PullRequestFile>> {
    // Cut the text at every file header, then read each section on its own.
    let mut sections: Vec<Vec<&str>> = Vec::new();
    for line in diff.lines() {
        match (line.starts_with(FILE_HEADER), sections.last_mut()) {
            (true, _) => sections.push(vec![line]),
            (false, Some(section)) => section.push(line),
            // Text before the first header (none in git's output) belongs to no file.
            (false, None) if line.trim().is_empty() => {}
            (false, None) => return None,
        }
    }
    sections
        .iter()
        .map(|section| file_of_section(section))
        .collect()
}

/// The file one section describes; `None` when no path can be read.
fn file_of_section(section: &[&str]) -> Option<PullRequestFile> {
    if section.len() < 2 {
        return None;
    }
    let first_hunk = section
        .iter()
        .position(|line| line.starts_with(HUNK_HEADER))
        .unwrap_or(section.len());
    let (headers, hunks) = section.split_at(first_hunk);

    // Read the extended headers: they say how the file changed and its paths.
    let mut header = SectionHeader::default();
    for line in headers {
        header.read(line)?;
    }

    // Count the changed lines of the hunks; header lines start with "@@".
    let additions = hunks.iter().filter(|line| line.starts_with('+')).count();
    let deletions = hunks.iter().filter(|line| line.starts_with('-')).count();
    let (filename, previous_filename) = header.paths(section[0])?;
    Some(PullRequestFile {
        status: header.kind.status().to_owned(),
        previous_filename: (matches!(header.kind, ChangeKind::Renamed | ChangeKind::Copied))
            .then_some(previous_filename)
            .flatten(),
        filename,
        additions: u64::try_from(additions).ok()?,
        deletions: u64::try_from(deletions).ok()?,
        patch: (!hunks.is_empty()).then(|| hunks.join("\n")),
        sha: header.new_blob.clone(),
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
    /// The abbreviated blob after the change, from `index OLD..NEW`; `None`
    /// for a deleted file (all zeros) or without the line.
    new_blob: Option<String>,
}

impl SectionHeader {
    /// Takes in one header line; lines it does not know (modes,
    /// `similarity index`, `Binary files …`) change nothing.
    fn read(&mut self, line: &str) -> Option<()> {
        if line.starts_with("new file mode") {
            self.kind = ChangeKind::Added;
        } else if line.starts_with("deleted file mode") {
            self.kind = ChangeKind::Removed;
        } else if let Some(path) = line.strip_prefix("rename from ") {
            self.kind = ChangeKind::Renamed;
            self.old_path = Some(unquoted(path)?);
        } else if let Some(path) = line.strip_prefix("rename to ") {
            self.kind = ChangeKind::Renamed;
            self.new_path = Some(unquoted(path)?);
        } else if let Some(path) = line.strip_prefix("copy from ") {
            self.kind = ChangeKind::Copied;
            self.old_path = Some(unquoted(path)?);
        } else if let Some(path) = line.strip_prefix("copy to ") {
            self.kind = ChangeKind::Copied;
            self.new_path = Some(unquoted(path)?);
        } else if let Some(path) = line.strip_prefix("--- ") {
            read_side_path(&mut self.old_path, path, "a/")?;
        } else if let Some(path) = line.strip_prefix("+++ ") {
            read_side_path(&mut self.new_path, path, "b/")?;
        } else if let Some(blobs) = line.strip_prefix(INDEX_HEADER) {
            self.new_blob = new_blob_of_index(blobs);
        }
        Some(())
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

/// The blob after the change in an `index OLD..NEW [mode]` line's
/// `OLD..NEW [mode]`; `None` when it is all zeros (the file was deleted).
fn new_blob_of_index(blobs: &str) -> Option<String> {
    let (_, new) = blobs.split_whitespace().next()?.split_once("..")?;
    let is_hex = !new.is_empty() && new.bytes().all(|byte| byte.is_ascii_hexdigit());
    let is_deleted = new.bytes().all(|byte| byte == b'0');
    (is_hex && !is_deleted).then(|| new.to_owned())
}

/// Read a side header, retaining extended rename/copy headers when present.
fn read_side_path(slot: &mut Option<String>, raw: &str, prefix: &str) -> Option<()> {
    let path = unquoted(raw.split('\t').next()?)?;
    if path != NO_FILE {
        slot.get_or_insert(path.strip_prefix(prefix)?.to_owned());
    }
    Some(())
}

/// The two paths of `diff --git a/OLD b/NEW`. Paths with spaces are
/// ambiguous here, so when the halves differ the split that gives two equal
/// paths wins (the common unchanged-path case), else the first ` b/`.
fn header_line_paths(line: &str) -> Option<(String, String)> {
    let paths = line.strip_prefix(FILE_HEADER)?;
    // Find a delimiter outside a quoted path. Unquoted paths may contain
    // spaces and even " b/"; identical halves identify ordinary changes.
    let mut candidate = None;
    for (i, _) in paths.match_indices(' ') {
        let (Some(old), Some(new)) = (unquoted(&paths[..i]), unquoted(&paths[i + 1..])) else {
            continue;
        };
        let (Some(old), Some(new)) = (old.strip_prefix("a/"), new.strip_prefix("b/")) else {
            continue;
        };
        if old == new {
            return Some((old.into(), new.into()));
        }
        candidate.get_or_insert_with(|| (old.into(), new.into()));
    }
    candidate
}

/// Decode Git's C-style quoted bytes before interpreting them as UTF-8.
fn unquoted(path: &str) -> Option<String> {
    if !path.starts_with('"') {
        return Some(path.to_owned());
    }
    let mut input = path.strip_prefix('"')?.strip_suffix('"')?.bytes();
    let mut out = Vec::new();
    while let Some(ch) = input.next() {
        out.push(match ch {
            b'"' => return None,
            b'\\' => match input.next()? {
                b'a' => 7,
                b'b' => 8,
                b't' => 9,
                b'n' => 10,
                b'v' => 11,
                b'f' => 12,
                b'r' => 13,
                c @ (b'"' | b'\\') => c,
                a @ b'0'..=b'3' => {
                    let b = input.next()?;
                    let c = input.next()?;
                    if !(b'0'..=b'7').contains(&b) || !(b'0'..=b'7').contains(&c) {
                        return None;
                    }
                    (a - b'0') * 64 + (b - b'0') * 8 + (c - b'0')
                }
                _ => return None,
            },
            c => c,
        });
    }
    String::from_utf8(out).ok()
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
        let files = files_of_unified_diff(DIFF).unwrap();
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

    /// The blob after the change comes from the `index` line; deleted files and renames without one have none.
    #[test]
    fn reads_the_new_blob_from_the_index_line() {
        let files = files_of_unified_diff(DIFF).unwrap();
        let blobs: Vec<Option<&str>> = files.iter().map(|file| file.sha.as_deref()).collect();
        assert_eq!(
            blobs,
            [
                Some("2222222"),
                Some("3333333"),
                None,
                None,
                Some("6666666")
            ]
        );
    }

    /// The patch starts at the first hunk header, like the files API's.
    #[test]
    fn patch_starts_at_first_hunk() {
        let files = files_of_unified_diff(DIFF).unwrap();
        assert_eq!(
            files[0].patch.as_deref(),
            Some("@@ -1,2 +1,2 @@\n fn main() {\n-    old();\n+    new();")
        );
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn handles_mixed_quoted_rename_and_space_terminator() {
        let diff = "diff --git a/source \"b/na\\303\\257ve\"\nrename from source\nrename to \"na\\303\\257ve\"\ndiff --git a/a b b/a b\n+++ b/a b\t\n@@ -1 +1 @@\n+x";
        let files = files_of_unified_diff(diff).unwrap();
        assert_eq!(files[0].filename, "naïve");
        assert_eq!(files[1].filename, "a b");
        assert!(files_of_unified_diff(&format!("{diff}\ndiff --git malformed")).is_none());
        assert!(
            files_of_unified_diff(&format!("{diff}\ndiff --git a/truncated b/truncated")).is_none()
        );
    }
}
