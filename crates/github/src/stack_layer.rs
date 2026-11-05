/// One pull request of a stack, with the branches and commits it compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackLayer {
    /// The pull request's id in the pulls API. Not the id the search and
    /// issues APIs give the same pull request; match layers by `number`.
    pub pull_request_id: u64,
    pub number: u64,
    pub title: String,
    pub is_open: bool,
    pub is_draft: bool,
    pub is_merged: bool,
    pub html_url: String,
    pub head_ref: String,
    pub head_sha: String,
    /// The branch this layer targets: the layer below's head, or the trunk.
    pub base_ref: String,
    pub base_sha: String,
    /// Empty when GitHub reports no author (a deleted account).
    pub author_login: String,
}
