use super::stack_key::StackKey;

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
}
