use std::ops::Range;

use crate::SyntaxKind;

/// A byte range of a line holding one kind of token. Text outside every
/// span is plain (variables, operators, punctuation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxSpan {
    pub range: Range<usize>,
    pub kind: SyntaxKind,
}
