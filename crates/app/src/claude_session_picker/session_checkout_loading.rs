use std::collections::BTreeSet;
use std::path::PathBuf;

use futures::{StreamExt as _, stream};
use gpui::Context;
use qd_claude::{GitBranch, GitRepository};

use super::claude_session_picker::ClaudeSessionPicker;
use super::session_group::SessionGroup;

const PARALLEL_CHECKOUT_LOOKUPS: usize = 4;

/// Optional metadata for one shared session directory.
struct CheckoutLookup {
    directory: PathBuf,
    branch: Option<GitBranch>,
    repository: Option<GitRepository>,
}

impl ClaudeSessionPicker {
    /// Adds repository groups and branches after discovery is visible, once per unique directory.
    /// Refresh drops this task so old results cannot decorate a new inventory.
    pub(super) fn load_session_checkouts(&mut self, cx: &mut Context<Self>) {
        let directories: BTreeSet<_> = self
            .sessions
            .iter()
            .map(|session| session.working_directory.clone())
            .collect();
        self.checkout_task = Some(cx.spawn(async move |picker, cx| {
            // Bound parallel Git processes; none of their reads run on the UI thread.
            let executor = cx.background_executor().clone();
            let lookups = stream::iter(directories)
                .map(|directory| {
                    executor.spawn(async move {
                        let repository = GitRepository::read(&directory);
                        let branch = GitBranch::read(&directory);
                        CheckoutLookup {
                            directory,
                            branch,
                            repository,
                        }
                    })
                })
                .buffer_unordered(PARALLEL_CHECKOUT_LOOKUPS)
                .collect::<Vec<_>>()
                .await;

            // Group once after metadata arrives so rows do not jump between partial groups.
            picker
                .update(cx, |picker, cx| {
                    for lookup in lookups {
                        for (index, session) in picker.sessions.iter().enumerate() {
                            if session.working_directory == lookup.directory {
                                if let Some(repository) = &lookup.repository {
                                    picker.presentation[index].set_repository(repository);
                                }
                                if let Some(branch) = &lookup.branch {
                                    picker.presentation[index].set_branch(branch);
                                }
                            }
                        }
                    }
                    picker.groups = SessionGroup::for_repositories(
                        picker.presentation.iter().map(|row| row.repository.clone()),
                    );
                    picker.selection.update_searchable(
                        picker
                            .presentation
                            .iter()
                            .map(|row| row.searchable.clone())
                            .collect(),
                        &picker.query,
                    );
                    picker.selection.set_order(
                        picker
                            .groups
                            .iter()
                            .flat_map(|group| group.sessions.iter().copied())
                            .collect(),
                        &picker.query,
                    );
                    picker.rebuild_session_rows(cx);
                    picker.checkout_task = None;
                })
                .ok();
        }));
    }
}
