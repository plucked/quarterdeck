use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::ops::RangeInclusive;

use qd_diff::FileDiff;

use super::code_anchor::CodeAnchor;
use super::code_side::CodeSide;

/// The most lines one anchor may span, so excerpts stay readable.
const MAX_ANCHOR_LINES: u32 = 80;

/// The line numbers a file's diff shows on each side.
#[derive(Default)]
struct ShownLines {
    base: BTreeSet<u32>,
    head: BTreeSet<u32>,
}

impl ShownLines {
    fn of_side(&self, side: CodeSide) -> &BTreeSet<u32> {
        match side {
            CodeSide::Base => &self.base,
            CodeSide::Head => &self.head,
        }
    }
}

/// Which lines of which files an anchor may point at: the lines the pull
/// request's diffs show, on each side. A file is found by its path and, for
/// a rename, by its previous path too.
#[derive(Default)]
pub struct AnchorTargets {
    files: HashMap<String, ShownLines>,
}

impl AnchorTargets {
    /// Adds the lines of `diff`, the diff of the file at `path` (previously
    /// at `previous_path` when renamed).
    pub fn add_file(&mut self, path: &str, previous_path: Option<&str>, diff: &FileDiff) {
        // Collect each side's line numbers from every hunk.
        let mut shown = ShownLines::default();
        for line in diff.hunks().iter().flat_map(|hunk| &hunk.lines) {
            shown.base.extend(line.old_number);
            shown.head.extend(line.new_number);
        }
        // A renamed file answers to its old path as well.
        if let Some(previous_path) = previous_path {
            self.files.insert(
                previous_path.to_owned(),
                ShownLines {
                    base: shown.base.clone(),
                    head: shown.head.clone(),
                },
            );
        }
        self.files.insert(path.to_owned(), shown);
    }

    /// Checks that `anchor`'s first and last line are shown in its file's
    /// diff on its side. Lines between them may fall in a gap between two
    /// hunks: the excerpt then shows the gap, never text the diff lacks.
    /// The error says what is shown instead, for the agent or reader.
    pub fn check(&self, anchor: &CodeAnchor) -> Result<(), String> {
        // The range itself must make sense.
        if anchor.first_line == 0 || anchor.first_line > anchor.last_line {
            return Err(format!("{anchor}: not a line range"));
        }
        if anchor.last_line - anchor.first_line >= MAX_ANCHOR_LINES {
            return Err(format!(
                "{anchor}: spans more than {MAX_ANCHOR_LINES} lines"
            ));
        }
        // The file must be one of the pull request's.
        let Some(shown) = self.files.get(&anchor.path) else {
            return Err(format!("{anchor}: not a changed file of this pull request"));
        };
        // Both ends must be in the diff on that side.
        let side_lines = shown.of_side(anchor.side);
        if side_lines.contains(&anchor.first_line) && side_lines.contains(&anchor.last_line) {
            return Ok(());
        }
        Err(format!(
            "{anchor}: its first or last line is not in the diff; {} lines shown: {}",
            anchor.side.as_str(),
            describe_ranges(side_lines)
        ))
    }
}

/// `12–20, 40–52`: the runs of consecutive numbers in `lines`.
fn describe_ranges(lines: &BTreeSet<u32>) -> String {
    let mut runs: Vec<RangeInclusive<u32>> = Vec::new();
    for &line in lines {
        match runs.last_mut() {
            Some(run) if *run.end() + 1 == line => *run = *run.start()..=line,
            Some(_) | None => runs.push(line..=line),
        }
    }
    if runs.is_empty() {
        return "none".to_owned();
    }
    let mut described = String::new();
    for (index, run) in runs.iter().enumerate() {
        if index > 0 {
            described.push_str(", ");
        }
        let _ = write!(described, "{}–{}", run.start(), run.end());
    }
    described
}

#[cfg(test)]
mod tests {
    use qd_diff::DiffOptions;

    use super::*;

    const PATCH: &str = "@@ -10,3 +10,4 @@\n a\n-b\n+c\n+d\n e";

    fn targets() -> AnchorTargets {
        let mut targets = AnchorTargets::default();
        targets.add_file(
            "src/lib.rs",
            Some("src/old.rs"),
            &FileDiff::build(PATCH, DiffOptions::default()),
        );
        targets
    }

    fn anchor(path: &str, side: CodeSide, first_line: u32, last_line: u32) -> CodeAnchor {
        CodeAnchor {
            path: path.to_owned(),
            side,
            first_line,
            last_line,
        }
    }

    /// Lines shown on the anchor's side are accepted, on either path of a rename.
    #[test]
    fn accepts_shown_lines() {
        let targets = targets();
        assert!(
            targets
                .check(&anchor("src/lib.rs", CodeSide::Head, 10, 13))
                .is_ok()
        );
        assert!(
            targets
                .check(&anchor("src/old.rs", CodeSide::Base, 10, 12))
                .is_ok()
        );
    }

    /// A range may step over a gap between hunks when both ends are shown.
    #[test]
    fn accepts_ranges_across_a_gap() {
        let mut targets = AnchorTargets::default();
        let patch = "@@ -1,2 +1,2 @@\n a\n-b\n+c\n@@ -9,1 +9,1 @@\n-x\n+y";
        targets.add_file(
            "src/two.rs",
            None,
            &FileDiff::build(patch, DiffOptions::default()),
        );
        assert!(
            targets
                .check(&anchor("src/two.rs", CodeSide::Head, 1, 9))
                .is_ok()
        );
        assert!(
            targets
                .check(&anchor("src/two.rs", CodeSide::Head, 1, 8))
                .is_err()
        );
    }

    /// An end outside the hunk is refused with the shown ranges.
    #[test]
    fn refuses_lines_outside_the_diff() {
        let error = targets()
            .check(&anchor("src/lib.rs", CodeSide::Head, 12, 14))
            .unwrap_err();
        assert!(error.contains("head lines shown: 10–13"), "{error}");
    }

    /// Unknown files and reversed ranges are refused.
    #[test]
    fn refuses_unknown_files_and_bad_ranges() {
        let targets = targets();
        assert!(
            targets
                .check(&anchor("src/other.rs", CodeSide::Head, 10, 10))
                .is_err()
        );
        assert!(
            targets
                .check(&anchor("src/lib.rs", CodeSide::Head, 12, 11))
                .is_err()
        );
    }
}
