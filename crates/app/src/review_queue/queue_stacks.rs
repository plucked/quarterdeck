use std::collections::{HashMap, HashSet};

use gpui::{App, Context};
use qd_github::PullRequestStack;

use super::ReviewQueue;
use super::queue_lists::QueueLists;
use super::selection_origin::SelectionOrigin;
use super::stack_key::StackKey;
use crate::fetch_stack::{cached_stack, refresh_stack};
use crate::pull_request_detail::PullRequestDetail;
use crate::shared_cache::SharedCache;

impl ReviewQueue {
    /// Checks the layers of every listed stack with GitHub (each request
    /// free when unchanged), then shows the stacks whole.
    pub(super) fn refresh_stacks(&mut self, cx: &mut Context<Self>) {
        let keys = listed_stack_keys(&self.lists);
        let Ok(client) = self.services.github.clone() else {
            return;
        };
        let cache = self.services.cache.clone();
        self.stacks_task = Some(cx.spawn(async move |queue, cx| {
            let refreshed = cx
                .background_executor()
                .spawn(async move {
                    keys.into_iter()
                        .filter_map(|key| {
                            match refresh_stack(Ok(client.clone()), &cache, &key.repo, key.number) {
                                Ok(stack) => Some((key, stack)),
                                Err(error) => {
                                    eprintln!(
                                        "stack {}#{}: {error:#}",
                                        key.repo.full_name(),
                                        key.number
                                    );
                                    None
                                }
                            }
                        })
                        .collect::<HashMap<_, _>>()
                })
                .await;
            queue
                .update(cx, |queue, cx| {
                    // Keep the last known layers of a stack whose check failed.
                    let listed = listed_stack_keys(&queue.lists);
                    queue.stacks.retain(|key, _| listed.contains(key));
                    queue.stacks.extend(refreshed);
                    queue.rebuild_rows();
                    cx.notify();
                })
                .ok();
        }));
    }

    /// The known layers of stack `key`.
    pub(super) fn stack(&self, key: &StackKey) -> Option<&PullRequestStack> {
        self.stacks.get(key)
    }

    /// Opens the combined review of a stack (a click on its header in the
    /// sidebar): its highest listed layer, combined from the lowest unmerged one.
    pub(super) fn open_stack(&mut self, key: &StackKey, cx: &mut Context<Self>) {
        let Some(top) = self
            .shown_list()
            .iter()
            .filter(|pull_request| StackKey::of(pull_request).as_ref() == Some(key))
            .max_by_key(|pull_request| {
                pull_request
                    .stack
                    .as_ref()
                    .map_or(0, |membership| membership.position)
            })
            .cloned()
        else {
            return;
        };

        // Already shown: only the review changes; else it is set before the new layer shows.
        let is_shown = self.selected.as_ref().map(|selected| selected.id) == Some(top.id);
        if is_shown {
            self.detail()
                .update(cx, PullRequestDetail::review_combined_stack);
            cx.notify();
            return;
        }
        self.detail()
            .update(cx, PullRequestDetail::prepare_combined_stack_review);
        self.select_pull_request(top, SelectionOrigin::Listed, cx);
    }

    /// Whether the detail shows the combined review of stack `key`, so its
    /// header is highlighted.
    pub(super) fn is_stack_reviewed(&self, key: &StackKey, cx: &App) -> bool {
        self.selected
            .as_ref()
            .and_then(StackKey::of)
            .is_some_and(|selected| &selected == key)
            && self.detail().read(cx).is_combined_review()
    }
}

/// The cached layers of the listed stacks; a stack not cached (or not
/// readable) is left out, and its listed layers show alone until checked.
pub fn read_cached_stacks(
    cache: &SharedCache,
    lists: &QueueLists,
) -> HashMap<StackKey, PullRequestStack> {
    listed_stack_keys(lists)
        .into_iter()
        .filter_map(|key| {
            let stack = cached_stack(cache, &key.repo, key.number).ok()??;
            Some((key, stack))
        })
        .collect()
}

/// The stacks the listed pull requests belong to, each once.
fn listed_stack_keys(lists: &QueueLists) -> HashSet<StackKey> {
    lists.iter().filter_map(StackKey::of).collect()
}
