/// A repository of the Others tab: how many of its open pull requests
/// there are before the filter, and the filter's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OthersGroup {
    pub total: usize,
    pub is_filter_active: bool,
    pub is_filter_open: bool,
}
