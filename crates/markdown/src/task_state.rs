/// The checkbox of a task list item (`- [ ]` or `- [x]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Open,
    Done,
}
