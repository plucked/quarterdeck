use std::time::{Duration, Instant};

use gpui::Context;
use qd_github::{MergeMethod, MergeOptions, MergeOutcome};

use super::action_panel::{ActionPanel, ActionProgress, ArmedAction, PanelAction};
use super::detail_event::DetailEvent;
use super::pull_request_detail::PullRequestDetail;

/// Time between looks at an asynchronous merge GitHub is still running.
const MERGE_POLL_INTERVAL: Duration = Duration::from_millis(1500);
/// How long a merge is followed before the panel stops waiting for it.
const MERGE_POLL_LIMIT: Duration = Duration::from_secs(180);

impl PullRequestDetail {
    /// Opens the merge or close panel and reads GitHub's options for the
    /// shown pull request (one GraphQL request); the same panel again closes it.
    pub(super) fn toggle_action_panel(&mut self, action: PanelAction, cx: &mut Context<Self>) {
        if self
            .action_panel()
            .is_some_and(|panel| panel.action == action && !panel.is_working())
        {
            self.set_action_panel(None);
            cx.notify();
            return;
        }
        let (Some(pull_request), Ok(client)) =
            (self.pull_request().cloned(), self.services().github.clone())
        else {
            return;
        };
        let mut panel = ActionPanel::new(action);
        panel.task = Some(cx.spawn(async move |detail, cx| {
            let options = cx
                .background_executor()
                .spawn(async move { client.merge_options(&pull_request.repo, pull_request.number) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    let can_delete = detail.can_delete_head_branch(options.as_ref().ok());
                    if let Some(panel) = detail.action_panel_mut() {
                        match options {
                            Ok(options) => {
                                panel.method = options.allowed_methods.first().copied();
                                // A merge leaves the branch to a repository that deletes it itself; a close never does.
                                panel.delete_branch = match panel.action {
                                    PanelAction::Merge => {
                                        can_delete && !options.deletes_branch_on_merge
                                    }
                                    PanelAction::Close => can_delete,
                                };
                                panel.options = Some(options);
                            }
                            Err(error) => panel.options_error = Some(format!("{error:#}").into()),
                        }
                    }
                    cx.notify();
                })
                .ok();
        }));
        self.set_action_panel(Some(panel));
        cx.notify();
    }

    /// Whether the head branch may be deleted with the action: the viewer may,
    /// it is not a fork's, and no open stack layer above is based on it.
    pub(super) fn can_delete_head_branch(&self, options: Option<&MergeOptions>) -> bool {
        let Some(options) = options else {
            return false;
        };
        options.permissions.can_delete_head_ref
            && !options.is_cross_repository
            && self.layer_above_head().is_none()
    }

    /// The number of an open stack layer based on the shown pull request's branch.
    pub(super) fn layer_above_head(&self) -> Option<u64> {
        let current = self.current_layer_index()?;
        let stack = self.stack()?;
        let head_ref = &stack.layers.get(current)?.head_ref;
        stack.layers[current + 1..]
            .iter()
            .find(|layer| layer.is_open && layer.base_ref == *head_ref)
            .map(|layer| layer.number)
    }

    /// Picks the merge method.
    pub(super) fn set_merge_method(&mut self, method: MergeMethod, cx: &mut Context<Self>) {
        if let Some(panel) = self.action_panel_mut() {
            panel.method = Some(method);
            panel.armed = None;
        }
        cx.notify();
    }

    /// Ticks or unticks deleting the head branch with the action.
    pub(super) fn toggle_delete_branch(&mut self, cx: &mut Context<Self>) {
        if let Some(panel) = self.action_panel_mut() {
            panel.delete_branch = !panel.delete_branch;
            panel.armed = None;
        }
        cx.notify();
    }

    /// Asks to confirm `action` (a first click); `None` steps back.
    pub(super) fn arm_action(&mut self, action: Option<ArmedAction>, cx: &mut Context<Self>) {
        if let Some(panel) = self.action_panel_mut() {
            panel.armed = action;
        }
        cx.notify();
    }

    /// Runs the confirmed action on GitHub: a merge (an admin merge bypasses
    /// rules), followed until GitHub finishes it, or closing; then deletes
    /// the head branch when asked. The queue refreshes after it.
    pub(super) fn run_armed_action(&mut self, cx: &mut Context<Self>) {
        let Some(panel) = self.action_panel() else {
            return;
        };
        let (Some(armed), Some(options), Some(pull_request), Ok(client)) = (
            panel.armed,
            panel.options.clone(),
            self.pull_request().cloned(),
            self.services().github.clone(),
        ) else {
            return;
        };
        let method = panel.method;
        let delete_branch = panel.delete_branch;
        let working = match armed {
            ArmedAction::Merge | ArmedAction::AdminMerge => "Merging on GitHub…",
            ArmedAction::Close => "Closing on GitHub…",
            ArmedAction::DeleteBranch => "Deleting the branch on GitHub…",
        };
        let delete_branch = delete_branch || armed == ArmedAction::DeleteBranch;
        let task = cx.spawn(async move |detail, cx| {
            let repo = pull_request.repo.clone();
            let number = pull_request.number;

            // The action itself: start a merge and follow it, or close.
            let mut outcome = match armed {
                ArmedAction::Merge | ArmedAction::AdminMerge => {
                    let Some(method) = method else {
                        return;
                    };
                    let (start_client, start_repo, head_sha) =
                        (client.clone(), repo.clone(), options.head_sha.clone());
                    let bypass = armed == ArmedAction::AdminMerge;
                    cx.background_executor()
                        .spawn(async move {
                            start_client.start_merge(&start_repo, number, method, &head_sha, bypass)
                        })
                        .await
                }
                // The pull request is finished already; only the branch goes.
                ArmedAction::DeleteBranch => Ok(MergeOutcome::Merged),
                ArmedAction::Close => {
                    let (close_client, close_repo) = (client.clone(), repo.clone());
                    cx.background_executor()
                        .spawn(async move { close_client.close_pull_request(&close_repo, number) })
                        .await
                        // A closed pull request is finished the way a merged one is.
                        .map(|()| MergeOutcome::Merged)
                }
            };

            // GitHub merges in the background: look again until it says how it ended.
            let started = Instant::now();
            while let Ok(MergeOutcome::Pending { uuid }) = &outcome {
                if started.elapsed() > MERGE_POLL_LIMIT {
                    break;
                }
                cx.background_executor().timer(MERGE_POLL_INTERVAL).await;
                let (poll_client, poll_repo, uuid) = (client.clone(), repo.clone(), uuid.clone());
                outcome = cx
                    .background_executor()
                    .spawn(async move { poll_client.merge_outcome(&poll_repo, number, &uuid) })
                    .await;
            }

            // The branch goes once the pull request is merged or closed (not while queued).
            let mut branch_error = None;
            if delete_branch && matches!(outcome, Ok(MergeOutcome::Merged)) {
                let (branch_client, branch) = (client.clone(), options.head_ref.clone());
                if let Err(error) = cx
                    .background_executor()
                    .spawn(async move { branch_client.delete_branch(&repo, &branch) })
                    .await
                {
                    branch_error = Some(format!("{error:#}"));
                }
            }

            detail
                .update(cx, |detail, cx| {
                    let progress = action_result(armed, &outcome, branch_error.as_deref());
                    let is_done = matches!(progress, ActionProgress::Done(_));
                    if let Some(panel) = detail.action_panel_mut() {
                        panel.progress = progress;
                        panel.armed = None;
                    }
                    if is_done {
                        cx.emit(DetailEvent::PullRequestChanged);
                    }
                    cx.notify();
                })
                .ok();
        });
        if let Some(panel) = self.action_panel_mut() {
            panel.progress = ActionProgress::Working(working.into());
            panel.task = Some(task);
        }
        cx.notify();
    }
}

/// What the panel says after `armed` ended with `outcome`.
fn action_result(
    armed: ArmedAction,
    outcome: &Result<MergeOutcome, qd_github::GitHubError>,
    branch_error: Option<&str>,
) -> ActionProgress {
    let branch_note = branch_error
        .map(|error| format!(" The branch could not be deleted: {error}"))
        .unwrap_or_default();
    match (armed, outcome) {
        (ArmedAction::DeleteBranch, Ok(_)) => match branch_error {
            Some(error) => {
                ActionProgress::Failed(format!("The branch could not be deleted: {error}").into())
            }
            None => ActionProgress::Done("Branch deleted.".into()),
        },
        (ArmedAction::Close, Ok(_)) => ActionProgress::Done(format!("Closed.{branch_note}").into()),
        (ArmedAction::Merge | ArmedAction::AdminMerge, Ok(MergeOutcome::Merged)) => {
            ActionProgress::Done(format!("Merged.{branch_note}").into())
        }
        (ArmedAction::Merge | ArmedAction::AdminMerge, Ok(MergeOutcome::Enqueued)) => {
            ActionProgress::Done(
                "Added to the merge queue; it merges when the queue reaches it.".into(),
            )
        }
        (ArmedAction::Merge | ArmedAction::AdminMerge, Ok(MergeOutcome::Pending { .. })) => {
            ActionProgress::Done("GitHub is still merging; refresh later to see the result.".into())
        }
        (ArmedAction::Merge | ArmedAction::AdminMerge, Ok(MergeOutcome::Failed(message))) => {
            ActionProgress::Failed(format!("GitHub did not merge it: {message}").into())
        }
        (_, Err(error)) => ActionProgress::Failed(format!("{error:#}").into()),
    }
}
