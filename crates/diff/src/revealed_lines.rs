use std::ops::Range;

/// The lines of a file's new side (numbers from 1) revealed around its
/// diff's hunks: sorted, non-overlapping, non-adjacent ranges. Kept by
/// line number, not by hunk, so they hold however the hunks merge.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RevealedLines {
    ranges: Vec<Range<u32>>,
}

impl RevealedLines {
    /// Adds the lines in `range` (end exclusive), merging with ranges it
    /// touches or overlaps.
    pub fn reveal(&mut self, range: Range<u32>) {
        if range.is_empty() {
            return;
        }
        // Ranges that end before `range` starts stay before it; those that start after it ends stay after.
        let first = self.ranges.partition_point(|kept| kept.end < range.start);
        let last = self.ranges.partition_point(|kept| kept.start <= range.end);
        let merged = self.ranges[first..last].iter().fold(range, |merged, kept| {
            merged.start.min(kept.start)..merged.end.max(kept.end)
        });
        self.ranges.splice(first..last, [merged]);
    }

    /// The revealed lines inside `lines`, as ranges in order.
    pub fn within(&self, lines: Range<u32>) -> impl Iterator<Item = Range<u32>> + '_ {
        // Skip the ranges that end before `lines`; stop at the first starting after it.
        let first = self
            .ranges
            .partition_point(|range| range.end <= lines.start);
        self.ranges[first..]
            .iter()
            .take_while(move |range| range.start < lines.end)
            .map(move |range| range.start.max(lines.start)..range.end.min(lines.end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Overlapping and touching ranges merge; apart ones stay apart.
    #[test]
    fn merges_touching_ranges() {
        let mut revealed = RevealedLines::default();
        revealed.reveal(10..20);
        revealed.reveal(30..40);
        revealed.reveal(20..25);
        assert_eq!(revealed.ranges, [10..25, 30..40]);
        revealed.reveal(5..35);
        assert_eq!(revealed.ranges, vec![5..40_u32]);
        revealed.reveal(3..3);
        assert_eq!(revealed.ranges, vec![5..40_u32]);
    }

    /// Only the revealed parts inside a span come back, clipped to it.
    #[test]
    fn lists_revealed_ranges_within_a_span() {
        let mut revealed = RevealedLines::default();
        revealed.reveal(10..12);
        revealed.reveal(20..30);
        revealed.reveal(40..41);
        let within: Vec<_> = revealed.within(11..25).collect();
        assert_eq!(within, [11..12, 20..25]);
        assert_eq!(revealed.within(12..20).count(), 0);
    }
}
