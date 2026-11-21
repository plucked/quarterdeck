use super::stack_key::StackKey;

/// Which relationships connect the group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackSource {
    GitHub,
    BranchMatches,
    Mixed,
}

impl StackSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub stack",
            Self::BranchMatches => "Branch matches",
            Self::Mixed => "GitHub + branch match",
        }
    }
}

/// What a stack's header shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackGroup {
    pub key: StackKey,
    /// Layers in the whole stack.
    pub size: u64,
    pub base_ref: String,
    /// Layers of it in the shown list.
    pub listed: usize,
    pub is_collapsed: bool,
    pub source: StackSource,
    pub branching: bool,
    pub root: u64,
}
