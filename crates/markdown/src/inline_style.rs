use std::ops::BitOr;

/// The styles applied to a run of inline text, combinable: `**[a](b)**` is
/// `STRONG | LINK`. Produced by the parser; the renderer maps each flag to
/// its look.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InlineStyle(u8);

impl InlineStyle {
    pub const PLAIN: Self = Self(0);
    pub const STRONG: Self = Self(1);
    pub const EMPHASIS: Self = Self(1 << 1);
    pub const STRIKETHROUGH: Self = Self(1 << 2);
    pub const CODE: Self = Self(1 << 3);
    pub const LINK: Self = Self(1 << 4);

    /// Whether every flag of `other` is set.
    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn is_plain(self) -> bool {
        self == Self::PLAIN
    }
}

impl BitOr for InlineStyle {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}
