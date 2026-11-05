use std::ops::Range;

use crate::SyntaxColor;

/// A byte range of a line drawn in a token's color and font style. Text
/// outside every span uses the theme's default foreground.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxSpan {
    pub range: Range<usize>,
    pub color: SyntaxColor,
    pub is_bold: bool,
    pub is_italic: bool,
}
