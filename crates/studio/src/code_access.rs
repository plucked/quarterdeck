/// What the agent can read besides the numbered diffs in its prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeAccess {
    /// Only the diffs; the reason the code is not there (shown to the
    /// reader with the brief).
    DiffOnly(String),
    /// A review workspace: `head/` with the head commit's files and
    /// `change.diff`, readable with `Read`, `Grep` and `Glob`.
    Workspace,
}
