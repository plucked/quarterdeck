/// The click count of a single press.
const SINGLE_CLICK: usize = 1;
/// The click count of a double click: it selects a word, and on a diff
/// line opens a comment.
pub const DOUBLE_CLICK: usize = 2;

/// What a press in the code selects, by its click count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionGranularity {
    /// One click: nothing yet; dragging selects.
    Drag,
    /// Two clicks: the word under the pointer.
    Word,
    /// Three or more: the whole line.
    Line,
}

impl SelectionGranularity {
    /// The granularity of a press with `click_count` clicks.
    pub fn of(click_count: usize) -> Self {
        match click_count {
            0 | SINGLE_CLICK => Self::Drag,
            DOUBLE_CLICK => Self::Word,
            _ => Self::Line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One click drags, two select a word, three or more the line.
    #[test]
    fn picks_the_granularity_by_click_count() {
        assert_eq!(SelectionGranularity::of(0), SelectionGranularity::Drag);
        assert_eq!(SelectionGranularity::of(1), SelectionGranularity::Drag);
        assert_eq!(SelectionGranularity::of(2), SelectionGranularity::Word);
        assert_eq!(SelectionGranularity::of(3), SelectionGranularity::Line);
        assert_eq!(SelectionGranularity::of(4), SelectionGranularity::Line);
    }
}
