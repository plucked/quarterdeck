use std::ops::Range;

use qd_diff::ContextGap;

/// Lines revealed per click, as GitHub does.
pub(super) const CONTEXT_STEP: u32 = 20;

/// Which lines of a gap a click reveals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapPart {
    /// The next lines below the hunk above.
    Top,
    /// The next lines above the hunk below.
    Bottom,
    /// The whole gap (to the end of the file for the last one).
    All,
}

impl GapPart {
    /// The new-side lines of `gap` this part reveals.
    pub fn lines_of(self, gap: &ContextGap) -> Range<u32> {
        match self {
            Self::Top => gap.top(CONTEXT_STEP),
            Self::Bottom => gap.bottom(CONTEXT_STEP),
            Self::All => gap.all(),
        }
    }
}
