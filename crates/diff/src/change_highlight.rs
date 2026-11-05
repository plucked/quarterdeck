/// How finely changes inside a changed line are marked.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ChangeHighlight {
    /// Only whole lines are marked as added or removed.
    Lines,
    /// Changed words are marked inside a changed line.
    #[default]
    Words,
    /// Changed characters are marked inside a changed line.
    Characters,
}
