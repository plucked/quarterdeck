/// What the detail view tells the view that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailEvent {
    /// The pane scrolled; its position is worth saving.
    Scrolled,
}
