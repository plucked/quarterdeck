use gpui::{SharedString, Task};
use qd_github::{MergeMethod, MergeOptions};

/// Which action the panel under the header prepares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelAction {
    Merge,
    Close,
}

/// An action the user asked for once and must confirm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmedAction {
    Merge,
    /// A merge past unmet requirements the viewer may bypass.
    AdminMerge,
    Close,
    /// Deleting the head branch of a pull request already closed or merged.
    DeleteBranch,
}

/// How the action is going.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ActionProgress {
    #[default]
    Idle,
    Working(SharedString),
    Done(SharedString),
    Failed(SharedString),
}

/// The merge or close panel under the header: GitHub's options for the
/// pull request (read when the panel opens), the user's choices, the
/// action waiting for confirmation, and how it is going.
pub struct ActionPanel {
    pub action: PanelAction,
    /// `None` while GitHub's options are being read.
    pub options: Option<MergeOptions>,
    pub options_error: Option<SharedString>,
    pub method: Option<MergeMethod>,
    pub delete_branch: bool,
    pub armed: Option<ArmedAction>,
    pub progress: ActionProgress,
    /// Reading the options, or running the action; dropping it cancels.
    pub task: Option<Task<()>>,
}

impl ActionPanel {
    pub fn new(action: PanelAction) -> Self {
        Self {
            action,
            options: None,
            options_error: None,
            method: None,
            delete_branch: false,
            armed: None,
            progress: ActionProgress::Idle,
            task: None,
        }
    }

    /// Whether an action is running (the buttons wait).
    pub fn is_working(&self) -> bool {
        matches!(self.progress, ActionProgress::Working(_))
    }
}
