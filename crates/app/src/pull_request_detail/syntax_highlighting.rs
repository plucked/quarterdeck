use std::sync::Arc;

use gpui::{Context, Task};
use qd_cache::DiffViewSettings;

use super::batch_progress::BatchProgress;
use super::changed_files::ChangedFiles;
use super::highlighted_diff::HighlightedDiff;
use super::pull_request_detail::PullRequestDetail;

/// Batches when the number of cores is unknown.
const DEFAULT_HIGHLIGHT_BATCHES: usize = 4;

impl PullRequestDetail {
    /// Applies diff settings changed in
    /// [`AppSettings`](crate::app_settings::AppSettings). Layout and
    /// wrapping rebuild the rows (heights change); whitespace and highlight
    /// rebuild the diffs off the main thread first.
    pub(super) fn apply_diff_settings(
        &mut self,
        settings: DiffViewSettings,
        cx: &mut Context<Self>,
    ) {
        let previous = self.diff_settings;
        self.diff_settings = settings;
        if previous.diff_options() == settings.diff_options() {
            if previous != settings {
                self.rebuild_rows(self.anchor_at_top());
            }
            cx.notify();
            return;
        }

        let files = self.loaded_files().to_vec();
        let options = settings.diff_options();
        self.diff_task = Some(cx.spawn(async move |detail, cx| {
            let rebuilt = cx
                .background_executor()
                .spawn(async move {
                    files
                        .iter()
                        .map(|loaded| loaded.rebuilt(options))
                        .collect::<Vec<_>>()
                })
                .await;
            // Only replace files that are still the ones shown.
            detail
                .update(cx, |detail, cx| {
                    let is_same_files = detail.loaded_files().len() == rebuilt.len()
                        && detail
                            .loaded_files()
                            .iter()
                            .zip(&rebuilt)
                            .all(|(shown, built)| shown.fingerprint == built.fingerprint);
                    if is_same_files {
                        let anchor = detail.anchor_at_top();
                        // Lines revealed while the rebuild ran stay revealed.
                        let rebuilt = rebuilt
                            .into_iter()
                            .zip(detail.loaded_files())
                            .map(|(built, shown)| built.keeping_context_of(shown))
                            .collect();
                        detail.changed_files = Some(ChangedFiles::Loaded(rebuilt));
                        detail.rebuild_rows(anchor);
                        detail.highlight_syntax(cx);
                        cx.notify();
                    }
                })
                .ok();
        }));
    }

    /// Colors the loaded diffs by language off the main thread, one batch of
    /// files per core, showing each batch's colors as it finishes. Files show
    /// plain until then, and colors change no row height, so rows stay.
    pub(super) fn highlight_syntax(&mut self, cx: &mut Context<Self>) {
        let files = self.loaded_files().to_vec();
        if files.is_empty() {
            self.syntax_task = None;
            self.highlight_progress = None;
            return;
        }

        // Start every batch at once; they run on the executor's threads.
        let cores =
            std::thread::available_parallelism().map_or(DEFAULT_HIGHLIGHT_BATCHES, usize::from);
        let batch_size = files.len().div_ceil(cores);
        let executor = cx.background_executor().clone();
        let batches: Vec<Task<Vec<HighlightedDiff>>> = files
            .chunks(batch_size)
            .map(|batch| {
                let batch = batch.to_vec();
                executor.spawn(async move { batch.into_iter().map(HighlightedDiff::of).collect() })
            })
            .collect();

        // Show batches in order, counting them for the indicator; dropping this task cancels the rest.
        let batch_count = batches.len();
        self.highlight_progress = Some(BatchProgress {
            done: 0,
            total: batch_count,
        });
        self.syntax_task = Some(cx.spawn(async move |detail, cx| {
            for (index, batch) in batches.into_iter().enumerate() {
                let highlighted = batch.await;
                let applied = detail.update(cx, |detail, cx| {
                    detail.apply_syntax(highlighted);
                    let done = index + 1;
                    detail.highlight_progress = (done < batch_count).then_some(BatchProgress {
                        done,
                        total: batch_count,
                    });
                    cx.notify();
                });
                if applied.is_err() {
                    return;
                }
            }
        }));
    }

    /// Sets the syntax colors of the files whose diffs are still the ones
    /// highlighted; a diff rebuilt meanwhile waits for its own pass.
    pub(super) fn apply_syntax(&mut self, highlighted: Vec<HighlightedDiff>) {
        let Some(files) = self.loaded_files_mut() else {
            return;
        };
        for result in highlighted {
            if let Some(loaded) = files
                .iter_mut()
                .find(|loaded| Arc::ptr_eq(&loaded.diff, &result.diff))
            {
                loaded.syntax = result.syntax;
            }
        }
    }
}
