use crate::BranchSide;

/// A rule hiding pull requests from every queue tab by branch name: those
/// whose branch on `side` matches `pattern`, a wildcard pattern where `*`
/// stands for any run of characters (`/` included) and `?` for one, e.g.
/// `feat/legacy-*`. A preference, never expired.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenBranchRule {
    /// As the user typed it, trimmed; never empty.
    pub pattern: String,
    pub side: BranchSide,
}
