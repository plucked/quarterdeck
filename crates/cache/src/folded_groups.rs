use qd_github::RepoRef;

/// The sidebar groups the user folded, kept across launches: repositories,
/// and stacks by their repository and number.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FoldedGroups {
    pub repositories: Vec<RepoRef>,
    pub stacks: Vec<FoldedStack>,
}

/// A folded stack: its repository and stack number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoldedStack {
    pub repo: RepoRef,
    pub number: u64,
}
