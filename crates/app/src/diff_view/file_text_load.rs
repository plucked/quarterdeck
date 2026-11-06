use gpui::SharedString;

/// How fetching a file's text for the lines around its hunks stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTextLoad {
    Loading,
    /// It could not be fetched or used; the message is shown at the gap.
    Failed(SharedString),
}
