use crate::{DiffHunk, DiffLine, DiffLineKind, LineOrigin};

/// What a tab shows as; tabs are expanded so highlight ranges match what is drawn.
pub(crate) const TAB_AS_SPACES: &str = "    ";

/// Parses the hunks of a unified patch as GitHub sends it per file (no
/// `---`/`+++` header): each `@@ -a,b +c,d @@` line starts a hunk, and its
/// lines are numbered from those starts. `\ No newline at end of file`
/// markers and lines before the first header are dropped. A malformed
/// header numbers its lines from 1.
pub(crate) fn parse_patch(patch: &str) -> Vec<DiffHunk> {
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut old_number = 1;
    let mut new_number = 1;

    for raw_line in patch.lines() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

        // A header starts a hunk and resets the line numbers.
        if line.starts_with("@@") {
            (old_number, new_number) = hunk_starts(line).unwrap_or((1, 1));
            hunks.push(DiffHunk {
                header: line.to_owned(),
                old_start: old_number,
                new_start: new_number,
                lines: Vec::new(),
            });
            continue;
        }
        let Some(hunk) = hunks.last_mut() else {
            continue;
        };

        // The first character says what the line is; the rest is its text.
        let mut characters = line.chars();
        let kind = match characters.next() {
            Some('+') => DiffLineKind::Added,
            Some('-') => DiffLineKind::Removed,
            Some('\\') => continue,
            Some(_) | None => DiffLineKind::Context,
        };
        let text = characters.as_str().replace('\t', TAB_AS_SPACES);
        let (old, new) = match kind {
            DiffLineKind::Context => (Some(old_number), Some(new_number)),
            DiffLineKind::Removed => (Some(old_number), None),
            DiffLineKind::Added => (None, Some(new_number)),
        };
        old_number += u32::from(old.is_some());
        new_number += u32::from(new.is_some());
        hunk.lines.push(DiffLine {
            kind,
            old_number: old,
            new_number: new,
            text,
            changed_ranges: Vec::new(),
            origin: LineOrigin::Patch,
        });
    }
    hunks
}

/// The old and new start lines of a `@@ -a,b +c,d @@` header.
fn hunk_starts(header: &str) -> Option<(u32, u32)> {
    let mut ranges = header.trim_start_matches('@').split_whitespace();
    let old = ranges.next()?.strip_prefix('-')?;
    let new = ranges.next()?.strip_prefix('+')?;
    let start = |range: &str| range.split(',').next()?.parse::<u32>().ok();
    Some((start(old)?, start(new)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Line numbers follow the header and advance per side.
    #[test]
    fn numbers_lines_from_header() {
        let hunks = parse_patch("@@ -10,3 +20,3 @@ fn main\n same\n-old\n+new\n");
        let numbers: Vec<_> = hunks[0]
            .lines
            .iter()
            .map(|line| (line.kind, line.old_number, line.new_number))
            .collect();
        assert_eq!(
            numbers,
            [
                (DiffLineKind::Context, Some(10), Some(20)),
                (DiffLineKind::Removed, Some(11), None),
                (DiffLineKind::Added, None, Some(21)),
            ]
        );
        assert_eq!(hunks[0].header, "@@ -10,3 +20,3 @@ fn main");
    }

    /// The no-newline marker is not a line, and tabs become spaces.
    #[test]
    fn drops_markers_and_expands_tabs() {
        let hunks = parse_patch("@@ -1 +1 @@\n-\tx\n\\ No newline at end of file\n+\ty\n");
        assert_eq!(hunks[0].lines.len(), 2);
        assert_eq!(hunks[0].lines[0].text, "    x");
    }

    /// A one-line range without a count still parses.
    #[test]
    fn parses_header_without_counts() {
        assert_eq!(hunk_starts("@@ -5 +7 @@"), Some((5, 7)));
        assert_eq!(hunk_starts("@@ nonsense @@"), None);
    }
}
