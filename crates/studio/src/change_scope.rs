/// What the change covers, each item a short Markdown line taken from the
/// diff: the behaviour it changes, what it deliberately leaves as it was,
/// and the mechanical edits a reader can skim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeScope {
    pub changes: Vec<String>,
    pub unchanged: Vec<String>,
    pub mechanical: Vec<String>,
}
