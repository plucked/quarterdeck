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

/// A confirmed command carries the head which was displayed when the user confirmed it.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ConfirmedAction {
    Merge {
        method: MergeMethod,
        head_sha: String,
        bypass: bool,
    },
    Close,
    DeleteBranch,
}

impl ConfirmedAction {
    pub(super) fn kind(&self) -> ArmedAction {
        match self {
            Self::Merge { bypass: true, .. } => ArmedAction::AdminMerge,
            Self::Merge { .. } => ArmedAction::Merge,
            Self::Close => ArmedAction::Close,
            Self::DeleteBranch => ArmedAction::DeleteBranch,
        }
    }
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

    pub(super) fn confirm(
        &mut self,
        shown_head: Option<&str>,
        may_delete: bool,
    ) -> Option<(ConfirmedAction, Option<String>)> {
        if self.is_working() {
            return None;
        }
        let options = self.options.as_ref()?;
        let armed = self.armed.take()?;
        let command = match armed {
            ArmedAction::Merge | ArmedAction::AdminMerge => {
                let Some((head, method)) = shown_head
                    .filter(|head| *head == options.head_sha)
                    .zip(self.method)
                else {
                    self.progress = OperationStatus::Failed(
                        "Refresh the pull request and review its current head before merging."
                            .into(),
                    );
                    return None;
                };
                ConfirmedAction::Merge {
                    method,
                    head_sha: head.into(),
                    bypass: armed == ArmedAction::AdminMerge,
                }
            }
            ArmedAction::Close => ConfirmedAction::Close,
            ArmedAction::DeleteBranch if may_delete => ConfirmedAction::DeleteBranch,
            ArmedAction::DeleteBranch => {
                self.progress = OperationStatus::Failed(
                    "Branch dependencies changed. Refresh before deleting this branch.".into(),
                );
                return None;
            }
        };
        let branch = (may_delete && (self.delete_branch || armed == ArmedAction::DeleteBranch))
            .then(|| options.head_ref.clone());
        Some((command, branch))
    }

    /// Whether an action is running (the buttons wait).
    pub fn is_working(&self) -> bool {
        matches!(self.progress, OperationStatus::Working(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qd_github::{MergePermissions, MergeQueueState, MergeReadiness, PullRequestState};

    fn armed(action: ArmedAction) -> ActionPanel {
        let mut panel = ActionPanel::new(PanelAction::Merge);
        panel.method = Some(MergeMethod::Squash);
        panel.armed = Some(action);
        panel.delete_branch = true;
        panel.options = Some(MergeOptions {
            state: PullRequestState::Open,
            readiness: MergeReadiness::Clean,
            allowed_methods: vec![MergeMethod::Squash],
            permissions: MergePermissions::default(),
            deletes_branch_on_merge: false,
            merge_queue: MergeQueueState::NoQueue,
            stack_number: None,
            head_ref: "feature".into(),
            head_sha: "updated".into(),
            base_ref: "main".into(),
            is_cross_repository: false,
        });
        panel
    }

    #[test]
    fn merges_only_the_displayed_commit_and_consumes_confirmation() {
        for kind in [ArmedAction::Merge, ArmedAction::AdminMerge] {
            for shown in [None, Some("old")] {
                let mut panel = armed(kind);
                assert!(panel.confirm(shown, true).is_none());
                assert!(panel.armed.is_none());
            }
            let mut panel = armed(kind);
            assert_eq!(
                panel.confirm(Some("updated"), true),
                Some((
                    ConfirmedAction::Merge {
                        method: MergeMethod::Squash,
                        head_sha: "updated".into(),
                        bypass: kind == ArmedAction::AdminMerge
                    },
                    Some("feature".into())
                ))
            );
            assert!(panel.confirm(Some("updated"), true).is_none());
        }
    }
    #[test]
    fn losing_delete_permission_after_arming_keeps_the_branch() {
        let mut close = armed(ArmedAction::Close);
        assert_eq!(
            close.confirm(None, false),
            Some((ConfirmedAction::Close, None))
        );
        assert!(
            armed(ArmedAction::DeleteBranch)
                .confirm(None, false)
                .is_none()
        );
        let mut working = armed(ArmedAction::Close);
        working.progress = OperationStatus::Working("Closing".into());
        assert!(working.confirm(None, true).is_none());
    }
}
