use gpui::{SharedString, Task};
use qd_github::{MergeMethod, MergeOptions};

use super::operation_status::OperationStatus;

/// Which action the Actions tab's panel prepares.
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

/// The merge or close panel in the side panel's Actions tab: GitHub's options for the
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
    /// How the action is going.
    pub progress: OperationStatus,
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
            progress: OperationStatus::Idle,
            task: None,
        }
    }

    /// Whether an action is running (the buttons wait).
    pub fn is_working(&self) -> bool {
        matches!(self.progress, OperationStatus::Working(_))
    }
}
