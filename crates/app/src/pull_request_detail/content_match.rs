/// A matching line in a complete source file, with a one-based line number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ContentMatch {
    pub line: u32,
    pub text: String,
    /// UTF-8 byte range of the first match inside the bounded excerpt.
    pub matched: std::ops::Range<usize>,
}
