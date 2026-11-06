use std::ops::Range;

/// Where a gap sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapPlace {
    /// Above hunk `n` (index into the diff's hunks); for hunk 0, the top of the file.
    AboveHunk(usize),
    /// Below the last hunk, to the end of the file.
    End,
}

/// Unchanged lines a diff does not show: above a hunk (between it and the
/// previous one, or the top of the file), or below the last hunk. The
/// lines are numbered on the new side; the old side's numbers are the
/// same lines shifted by `old_offset`, since nothing changes in a gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextGap {
    pub place: GapPlace,
    /// First hidden line of the new side.
    pub new_start: u32,
    /// One past the last hidden line; `None` below the last hunk while the
    /// file's length is unknown.
    pub new_end: Option<u32>,
    /// Old line number minus new line number, for every line of the gap.
    pub old_offset: i64,
}

impl ContextGap {
    /// How many lines the gap hides; `None` while its end is unknown.
    pub fn line_count(&self) -> Option<u32> {
        self.new_end.map(|end| end.saturating_sub(self.new_start))
    }

    /// The first `count` lines of the gap (those right below the hunk above).
    pub fn top(&self, count: u32) -> Range<u32> {
        let end = self.new_start.saturating_add(count);
        self.new_start..self.new_end.map_or(end, |gap_end| end.min(gap_end))
    }

    /// The last `count` lines of the gap (those right above the hunk below);
    /// the whole gap when its end is unknown.
    pub fn bottom(&self, count: u32) -> Range<u32> {
        match self.new_end {
            Some(end) => end.saturating_sub(count).max(self.new_start)..end,
            None => self.top(count),
        }
    }

    /// Every line of the gap; to the largest line number while its end is unknown.
    pub fn all(&self) -> Range<u32> {
        self.new_start..self.new_end.unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GAP: ContextGap = ContextGap {
        place: GapPlace::AboveHunk(1),
        new_start: 10,
        new_end: Some(50),
        old_offset: 2,
    };

    /// The top and bottom parts stay inside the gap.
    #[test]
    fn parts_stay_inside_the_gap() {
        assert_eq!(GAP.line_count(), Some(40));
        assert_eq!(GAP.top(20), 10..30);
        assert_eq!(GAP.bottom(20), 30..50);
        assert_eq!(GAP.top(100), 10..50);
        assert_eq!(GAP.bottom(100), 10..50);
    }

    /// A gap of unknown length grows downward from its start.
    #[test]
    fn open_gap_grows_from_its_start() {
        let open = ContextGap {
            place: GapPlace::End,
            new_end: None,
            ..GAP
        };
        assert_eq!(open.line_count(), None);
        assert_eq!(open.top(20), 10..30);
        assert_eq!(open.bottom(20), 10..30);
    }
}
