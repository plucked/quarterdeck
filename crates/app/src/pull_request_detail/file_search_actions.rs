use std::time::Duration;

use futures::{StreamExt as _, channel::mpsc};
use gpui::{Context, ScrollStrategy};

use super::file_search::{content_matches, fuzzy_path};
use super::pull_request_detail::PullRequestDetail;
use super::run_cancel::RunCancel;
use super::search_content_job::SearchContentJob;
use super::search_content_result::SearchContentResult;
use super::search_scope::SearchScope;
use crate::text_area::TextAreaEvent;

const SEARCH_DELAY: Duration = Duration::from_millis(120);

impl PullRequestDetail {
    /// Handles typing without scanning file contents on the window thread.
    pub(super) fn on_file_search(&mut self, event: TextAreaEvent, cx: &mut Context<Self>) {
        match event {
            TextAreaEvent::Changed => {
                self.file_tree.search.query = self.focus_drawer.input.read(cx).text().to_owned();
                self.schedule_file_search(cx);
            }
            TextAreaEvent::Cancel => {
                self.focus_drawer
                    .input
                    .update(cx, |input, cx| input.set_text(String::new(), cx));
            }
            TextAreaEvent::Submit => {
                if let Some(file) = self.focus_drawer.search_files.first().copied() {
                    self.show_file_in_diff(file, cx);
                }
            }
        }
        cx.notify();
    }

    /// Opens or closes the optional facets below the two fixed header rows.
    pub(super) fn toggle_filters_drawer(&mut self, cx: &mut Context<Self>) {
        self.focus_drawer.filters_open = !self.focus_drawer.filters_open;
        cx.notify();
    }

    /// Contents are loaded lazily when that scope has a query, and reused for subsequent queries.
    pub(super) fn set_file_search_scope(&mut self, scope: SearchScope, cx: &mut Context<Self>) {
        self.file_tree.search.scope = scope;
        self.schedule_file_search(cx);
        cx.notify();
    }

    /// Debounces typing, computes matches off-thread and replaces the visible navigation.
    pub(super) fn schedule_file_search(&mut self, cx: &mut Context<Self>) {
        self.focus_drawer.query_task = None;
        let query = self.file_tree.search.query.trim().to_owned();
        let scope = self.file_tree.search.scope;
        if query.is_empty() {
            self.file_tree.search.matches.clear();
            self.rebuild_rows(self.anchor_at_top());
            cx.notify();
            return;
        }
        if scope == SearchScope::Contents {
            self.start_content_search(cx);
        }
        let paths: Vec<_> = self
            .loaded_files()
            .iter()
            .map(|file| file.filename().to_owned())
            .collect();
        let contents = self.focus_drawer.contents.clone();
        self.focus_drawer.query_task = Some(cx.spawn(async move |detail, cx| {
            cx.background_executor().timer(SEARCH_DELAY).await;
            let matches = cx
                .background_executor()
                .spawn(async move {
                    paths
                        .into_iter()
                        .filter_map(|path| {
                            let found = match scope {
                                SearchScope::Paths => fuzzy_path(&path, &query),
                                SearchScope::Contents => contents
                                    .get(&path)
                                    .and_then(|text| content_matches(text, &query)),
                            };
                            found.map(|found| (path, found))
                        })
                        .collect()
                })
                .await;
            detail
                .update(cx, |detail, cx| {
                    detail.file_tree.search.matches = matches;
                    detail.rebuild_rows(detail.anchor_at_top());
                    detail
                        .focus_drawer
                        .search_scroll
                        .scroll_to_item(0, ScrollStrategy::Top);
                    cx.notify();
                })
                .ok();
        }));
        cx.notify();
    }

    /// Retries incomplete indexing after GitHub's requested cooldown, preserving viewed marks.
    pub(super) fn retry_content_search(&mut self, cx: &mut Context<Self>) {
        if self.focus_drawer.content_task.is_some() || self.focus_drawer.retry_task.is_some() {
            return;
        }
        self.focus_drawer.reset_contents();
        self.schedule_file_search(cx);
    }

    /// A single deadline re-enables Retry without polling or issuing another request.
    fn wait_for_search_retry(&mut self, delay: Duration, cx: &mut Context<Self>) {
        self.focus_drawer.retry_task = Some(cx.spawn(async move |detail, cx| {
            cx.background_executor().timer(delay).await;
            detail
                .update(cx, |detail, cx| {
                    detail.focus_drawer.retry_task = None;
                    cx.notify();
                })
                .ok();
        }));
        cx.notify();
    }

    /// Starts one cancellable, cache-first index for the loaded revision.
    fn start_content_search(&mut self, cx: &mut Context<Self>) {
        if self.focus_drawer.content_started || self.loaded_files().is_empty() {
            return;
        }
        let (Some(pull_request), Some(head)) = (self.pull_request(), self.shown_head_sha()) else {
            return;
        };
        let job = SearchContentJob {
            services: self.services().clone(),
            repo: pull_request.repo.clone(),
            files: self
                .loaded_files()
                .iter()
                .map(|file| file.file.clone())
                .collect(),
            head: head.into(),
            base: self.shown_base_sha.clone(),
        };
        self.focus_drawer.content_started = true;
        let (cancel, flag) = RunCancel::new();
        let task = cx.spawn(async move |detail, cx| {
            let (sender, mut receiver) = mpsc::unbounded();
            let load = cx.background_executor().spawn(async move {
                job.run(&flag, |result| {
                    sender.unbounded_send(result).ok();
                });
            });
            while let Some(result) = receiver.next().await {
                if detail
                    .update(cx, |detail, cx| {
                        match result {
                            SearchContentResult::Text { path, text } => {
                                detail.focus_drawer.contents.insert(path, text);
                                detail.focus_drawer.indexed += 1;
                            }
                            SearchContentResult::Skipped => detail.focus_drawer.skipped += 1,
                            SearchContentResult::Failed(error) => {
                                detail.focus_drawer.failed += 1;
                                detail.focus_drawer.content_error = Some(error);
                            }
                            SearchContentResult::Stopped {
                                message,
                                retry_after,
                            } => {
                                detail.focus_drawer.content_error = Some(message);
                                if let Some(delay) = retry_after {
                                    detail.wait_for_search_retry(delay, cx);
                                }
                            }
                        }
                        // Batches amortize large diff/tree rebuilds while results arrive.
                        if (detail.focus_drawer.indexed
                            + detail.focus_drawer.skipped
                            + detail.focus_drawer.failed)
                            % 20
                            == 0
                        {
                            detail.schedule_file_search(cx);
                        }
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
            load.await;
            detail
                .update(cx, |detail, cx| {
                    detail.focus_drawer.content_task = None;
                    detail.schedule_file_search(cx);
                    cx.notify();
                })
                .ok();
        });
        self.focus_drawer.content_task = Some((task, cancel));
        cx.notify();
    }
}
