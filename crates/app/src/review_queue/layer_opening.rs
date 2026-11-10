use gpui::{Context, Task};
use qd_github::{GitHubClient, PullRequest};

use super::ReviewQueue;
use super::selection_origin::SelectionOrigin;
use super::stack_key::StackKey;
use crate::pull_request_detail::LayerRequest;
use crate::shared_cache::SharedCache;

/// A stack layer being fetched because it is in no list.
pub struct OpeningLayer {
    pub number: u64,
    pub _task: Task<()>,
}

impl ReviewQueue {
    /// Shows the stack layer the detail asked for: from a list when listed,
    /// otherwise fetched from GitHub (one request) and cached.
    pub(super) fn open_layer(&mut self, request: &LayerRequest, cx: &mut Context<Self>) {
        if let Some(listed) = self
            .lists
            .find_by_number(&request.repo, request.number)
            .cloned()
        {
            self.select_pull_request(listed, SelectionOrigin::Listed, cx);
            return;
        }
        let Ok(client) = self.services.github.clone() else {
            self.layer_error = Some("Opening a layer that is in no list needs GitHub.".into());
            cx.notify();
            return;
        };
        let cache = self.services.cache.clone();
        let request = request.clone();
        let number = request.number;
        self.layer_error = None;
        let task = cx.spawn(async move |queue, cx| {
            let fetched = cx
                .background_executor()
                .spawn(async move { fetch_layer(&client, &cache, &request) })
                .await;
            queue
                .update(cx, |queue, cx| {
                    queue.opening_layer = None;
                    match fetched {
                        Ok(pull_request) => {
                            queue.select_pull_request(
                                pull_request,
                                SelectionOrigin::StackLayer,
                                cx,
                            );
                        }
                        Err(error) => {
                            queue.layer_error =
                                Some(format!("Cannot open #{number}: {error:#}").into());
                        }
                    }
                    cx.notify();
                })
                .ok();
        });
        self.opening_layer = Some(OpeningLayer {
            number,
            _task: task,
        });
        cx.notify();
    }

    /// Shows a layer of stack `key` that the shown list lacks (a click on its row).
    pub(super) fn open_other_layer(
        &mut self,
        key: &StackKey,
        layer: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = self
            .stacks
            .get(key)
            .and_then(|stack| LayerRequest::of_layer(stack, layer))
        else {
            return;
        };
        self.open_layer(&request, cx);
    }
}

/// Fetches a stack layer that is in no list, with the membership the stack
/// gave it, and caches it so a relaunch can show it again.
///
/// Blocking: runs on the background executor.
fn fetch_layer(
    client: &GitHubClient,
    cache: &SharedCache,
    request: &LayerRequest,
) -> anyhow::Result<PullRequest> {
    let mut pull_request = client.pull_request_by_number(&request.repo, request.number)?;
    pull_request.stack = Some(request.membership.clone());
    cache.lock().put_pull_request(&pull_request)?;
    Ok(pull_request)
}
