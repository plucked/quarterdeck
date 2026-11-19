use std::ops::RangeInclusive;

use crate::RawPatchLine;

/// The exact head-side lines in one hunk, joined by LF without a final
/// terminator. Unlike the display diff, this retains tabs and all whitespace.
pub fn patch_head_lines(patch: &str, range: RangeInclusive<u32>) -> Option<String> {
    let (&first, &last) = (range.start(), range.end());
    if first == 0 || first > last {
        return None;
    }
    // Only additions and shared context have a head-side source line.
    let mut lines = RawPatchLine::in_patch(patch).filter(|line| line.new_number.is_some());
    let start = lines.find(|line| line.new_number == Some(first))?;
    let mut source = start.source[1..].to_owned();
    // A replacement must fit wholly inside a single original hunk.
    for previous in first..last {
        let next = lines.next()?;
        if next.hunk != start.hunk || next.new_number != Some(previous + 1) {
            return None;
        }
        source.push('\n');
        source.push_str(&next.source[1..]);
    }
    Some(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads additions and context exactly, skipping deletions and EOF markers.
    #[test]
    fn keeps_patch_source_whitespace_and_empty_lines() {
        let patch = "@@ -9,3 +20,3 @@ section\r\n \tcafé  \r\n-removed\r\n\\ No newline at end of file\r\n+\r\n \t\r\n";
        assert_eq!(
            patch_head_lines(patch, 20..=22).as_deref(),
            Some("\tcafé  \n\n\t")
        );
        assert_eq!(patch_head_lines(patch, 21..=21).as_deref(), Some(""));
    }

    /// Missing lines, deleted-only hunks and even adjacent separate hunks have no suggestion range.
    #[test]
    fn requires_every_line_in_one_hunk() {
        let patch = "@@ -1 +1 @@\n+a\n@@ -2 +2 @@\n+b\n@@ -3 +3,0 @@\n-removed";
        assert_eq!(patch_head_lines(patch, 2..=2).as_deref(), Some("b"));
        for range in [0..=1, 1..=2, 2..=3, 3..=3] {
            assert_eq!(patch_head_lines(patch, range), None);
        }
        assert_eq!(patch_head_lines("@@ malformed @@\n+x", 1..=1), None);
        assert_eq!(patch_head_lines("@@ -1 +1 @@\n?x\n+y", 1..=1), None);
    }
}
