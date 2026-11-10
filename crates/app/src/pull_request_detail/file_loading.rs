use std::sync::Mutex;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::Context;
use qd_cache::{ReviewScope, UiStateStore, ViewedFile};
use qd_file_tree::FileTree;
use qd_github::{GitHubClient, PullRequest, PullRequestFile};

use super::brief_key::BriefKey;
use super::changed_files::ChangedFiles;
use super::context_loads::ContextLoads;
use super::fetch_changed_files::fetch_changed_files;
use super::fetch_combined_files::fetch_combined_files;
use super::file_review_state::FileReviewState;
use super::head_commit::HeadCommit;
use super::load_progress::LoadProgress;
use super::loaded_file::LoadedFile;
use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use super::scroll_anchor::ScrollAnchor;
use super::side_panel_tab::SidePanelTab;
use crate::brief_view::PreparedBrief;
use crate::shared_cache::SharedCache;
use crate::ui_state::{UiStateRecorder, read_brief, read_review_notes, read_viewed_files};

/// A download is reported again only after this many more bytes, so the
/// pane redraws a few times a second rather than for every chunk.
const DOWNLOAD_REPORT_BYTES: u64 = 256 * 1024;

/// A pull request's changed files as loaded on the background executor,
/// with the viewed marks saved for it.
struct FetchedChanges {
    /// In tree order: `tree`'s file indexes are positions here.
    files: Vec<LoadedFile>,
    tree: FileTree,
    viewed_marks: Vec<ViewedFile>,
    /// Whose marks those are, for marks set from now on.
    scope: ReviewScope,
    /// The user's review notes of that scope, and the GitHub review's summary.
    notes: Vec<qd_cache::ReviewNote>,
    summary: Option<qd_cache::ReviewSummary>,
    /// The brief stored for that scope, prepared for drawing.
    brief: Option<PreparedBrief>,
    /// The head commit the shown files are of, when known.
    head_sha: Option<String>,
}

impl PullRequestDetail {
    /// Loads the files of the review target: the pull request's (from the
    /// cache when current, otherwise from GitHub) or a combined range's (from
    /// the cache, or one diff download), caching what GitHub sent. Shows the
    /// progress while it runs; a combined review still waiting for the stack
    /// only shows that.
    pub(super) fn load_changed_files(&mut self, cx: &mut Context<Self>) {
        let Some(pull_request) = self.pull_request.clone() else {
            return;
        };
        let target = self.target.clone();
        let scope = target.viewed_scope(&pull_request);
        // A brief this session already holds there wins; the stored one is not read.
        let brief_key = BriefKey {
            pull_request_id: pull_request.id,
            target: target.clone(),
        };
        let wants_stored_brief = self.brief().state_of(&brief_key).is_none();
        let cache = self.services.cache.clone();
        let client = self.services.github.clone();
        let ui_state = cx.global::<UiStateRecorder>().shared_store();
        let options = self.diff_settings.diff_options();

        // Clear what was shown; a combined review without its layers only waits.
        let first_step = match target {
            ReviewTarget::AwaitingStack => LoadProgress::LoadingStack,
            ReviewTarget::Layer | ReviewTarget::Combined(_) => LoadProgress::ReadingCache,
        };
        self.changed_files = Some(ChangedFiles::Loading(first_step));
        self.file_review = FileReviewState::default();
        self.viewed_scope = None;
        // The head is known again with the files; until then no brief is compared with it.
        self.shown_head_sha = None;
        self.reset_review();
        self.syntax_task = None;
        self.context_loads = ContextLoads::default();
        self.highlight_progress = None;
        self.rebuild_rows(ScrollAnchor::Top);
        cx.notify();
        if target == ReviewTarget::AwaitingStack {
            self.files_task = None;
            return;
        }

        self.files_task = Some(cx.spawn(async move |detail, cx| {
            // Load on the background executor, reporting progress through a channel.
            let (progress_sender, mut progress) = mpsc::unbounded();
            let load = cx.background_executor().spawn(async move {
                let report = |step| {
                    progress_sender.unbounded_send(step).ok();
                };
                let files = fetch_target_files(client, &cache, &pull_request, &target, &report)?;

                // Build every diff here too: word comparisons can take a moment on big files.
                report(LoadProgress::BuildingDiffs { files: files.len() });
                let built: Vec<LoadedFile> = files
                    .into_iter()
                    .map(|file| LoadedFile::build(file, options))
                    .collect();

                // Put the files in tree order, so the tree and the diff agree.
                let paths: Vec<&str> = built.iter().map(LoadedFile::filename).collect();
                let ordered: Vec<LoadedFile> = FileTree::build(&paths)
                    .file_order()
                    .into_iter()
                    .map(|index| built[index].clone())
                    .collect();
                let ordered_paths: Vec<&str> = ordered.iter().map(LoadedFile::filename).collect();
                let tree = FileTree::build(&ordered_paths);
                let notes_and_summary = read_review_notes(&ui_state, &scope);

                // The stored brief of this scope, and the head the files are of.
                let brief = wants_stored_brief
                    .then(|| read_prepared_brief(&ui_state, &scope))
                    .flatten();
                let head_sha = HeadCommit::of(&target, &pull_request)
                    .resolve(&cache)
                    .ok()
                    .flatten();
                anyhow::Ok(FetchedChanges {
                    files: ordered,
                    tree,
                    viewed_marks: read_viewed_files(&ui_state, &scope),
                    notes: notes_and_summary.0,
                    summary: notes_and_summary.1,
                    scope,
                    brief,
                    head_sha,
                })
            });

            // Show each step as it is reported; the channel ends with the load.
            while let Some(step) = progress.next().await {
                let shown = detail.update(cx, |detail, cx| {
                    if let Some(ChangedFiles::Loading(current)) = detail.changed_files.as_mut() {
                        *current = step;
                        cx.notify();
                    }
                });
                if shown.is_err() {
                    return;
                }
            }
            let loaded = load.await;

            // Show the files at the pending position, then color them; a closed view drops it.
            detail
                .update(cx, |detail, cx| {
                    detail.changed_files = Some(match loaded {
                        Ok(fetched) => detail.take_fetched_changes(fetched, cx),
                        Err(error) => ChangedFiles::Failed(format!("{error:#}").into()),
                    });
                    detail.rebuild_rows(ScrollAnchor::Top);
                    detail.highlight_syntax(cx);
                    detail.load_github_review(cx);
                    cx.notify();
                })
                .ok();
        }));
    }

    /// Takes in what a load fetched: the viewed marks, the review notes and
    /// summary, the head commit, the stored brief and the file tree; returns
    /// the loaded files.
    fn take_fetched_changes(
        &mut self,
        fetched: FetchedChanges,
        cx: &mut Context<Self>,
    ) -> ChangedFiles {
        self.file_review = FileReviewState::for_loaded(&fetched.files, &fetched.viewed_marks);
        self.viewed_scope = Some(fetched.scope);
        self.review.comments.notes = fetched.notes;
        self.review.comments.summary = fetched.summary.unwrap_or_default();
        self.shown_head_sha = fetched.head_sha;
        if let Some(brief) = fetched.brief {
            self.show_stored_brief(brief);
        }
        // A summary editor opened while loading holds no saved summary; it
        // is made again with the loaded one, so typing never overwrites it.
        self.review.drop_summary_editor();
        if self.review.tab == SidePanelTab::Review {
            self.show_side_panel_tab(SidePanelTab::Review, cx);
        }
        self.file_tree.reset_for(fetched.tree);
        ChangedFiles::Loaded(fetched.files)
    }
}

/// Fetches the files of `target` for `pull_request`, reporting each step;
/// downloads are reported in steps of [`DOWNLOAD_REPORT_BYTES`].
///
/// Blocking: runs on the background executor.
fn fetch_target_files(
    client: Result<GitHubClient, String>,
    cache: &SharedCache,
    pull_request: &PullRequest,
    target: &ReviewTarget,
    report: &dyn Fn(LoadProgress),
) -> anyhow::Result<Vec<PullRequestFile>> {
    match target {
        ReviewTarget::Layer | ReviewTarget::AwaitingStack => {
            fetch_changed_files(client, cache, pull_request, &mut |step| report(step))
        }
        ReviewTarget::Combined(range) => {
            let mut reported_bytes = 0;
            fetch_combined_files(client, cache, range, &mut |download| {
                let is_done = download.total_bytes == Some(download.received_bytes);
                if is_done || download.received_bytes >= reported_bytes + DOWNLOAD_REPORT_BYTES {
                    reported_bytes = download.received_bytes;
                    report(LoadProgress::DownloadingDiff(download));
                }
            })
        }
    }
}

/// The brief stored for `scope`, prepared for drawing (Markdown parsed,
/// diagrams laid out); `None` without one. Blocks: background executor.
fn read_prepared_brief(store: &Mutex<UiStateStore>, scope: &ReviewScope) -> Option<PreparedBrief> {
    read_brief(store, scope).map(|stored| PreparedBrief::prepare(stored.studio, stored.record))
}
