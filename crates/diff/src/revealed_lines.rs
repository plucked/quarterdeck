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

    /// Whether line `number` is revealed.
    pub fn contains(&self, number: u32) -> bool {
        let index = self.ranges.partition_point(|range| range.end <= number);
        self.ranges
            .get(index)
            .is_some_and(|range| range.contains(&number))
    }

    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
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

    /// Only lines inside a range are revealed.
    #[test]
    fn contains_lines_inside_ranges() {
        let mut revealed = RevealedLines::default();
        revealed.reveal(10..12);
        revealed.reveal(20..21);
        let shown: Vec<u32> = (0..25).filter(|line| revealed.contains(*line)).collect();
        assert_eq!(shown, [10, 11, 20]);
    }
}
