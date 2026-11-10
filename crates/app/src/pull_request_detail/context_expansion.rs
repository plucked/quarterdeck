use std::sync::Arc;

use gpui::Context;
use qd_diff::{GapPlace, SourceLines};

use super::fetch_file_text::fetch_file_text;
use super::head_commit::HeadCommit;
use super::highlighted_diff::HighlightedDiff;
use super::loaded_file::{FileContext, LoadedFile};
use super::pull_request_detail::PullRequestDetail;
use crate::diff_view::{FileTextLoad, GapPart};

impl PullRequestDetail {
    /// Reveals part `part` of the hidden lines at `place` in file `file`.
    /// The first time, the file's text is fetched (from the cache, else one
    /// request), then the lines show; the file's rows alone are rebuilt, and
    /// its colors are worked out again in the background.
    pub(super) fn expand_context(
        &mut self,
        file: usize,
        place: GapPlace,
        part: GapPart,
        cx: &mut Context<Self>,
    ) {
        let Some(loaded) = self.loaded_files().get(file) else {
            return;
        };
        let Some(gap) = loaded.gap_at(place) else {
            return;
        };
        let lines = part.lines_of(gap);

        // The text is here: reveal at once.
        if let Some(context) = loaded.context() {
            let mut context = context.clone();
            context.revealed.reveal(lines);
            self.show_context(file, context, cx);
            return;
        }
        if self.context_loads().load_of(file) == Some(&FileTextLoad::Loading) {
            self.context_loads_mut().reveal_when_loaded(file, lines);
            return;
        }

        // Fetch the text in the background, then reveal the lines asked for.
        let Some(pull_request) = self.pull_request() else {
            return;
        };
        let head = HeadCommit::of(self.target(), pull_request);
        let repo = pull_request.repo.clone();
        let changed_file = loaded.file.clone();
        let patch_diff = loaded.patch_diff().clone();
        let client = self.services().github.clone();
        let cache = self.services().cache.clone();
        let task = cx.spawn(async move |detail, cx| {
            let fetched_file = changed_file.clone();
            let fetched = cx
                .background_executor()
                .spawn(async move {
                    let is_text_of_file = |source: &SourceLines| patch_diff.is_patch_of(source);
                    fetch_file_text(
                        client,
                        &cache,
                        &repo,
                        &fetched_file,
                        &head,
                        &is_text_of_file,
                    )
                })
                .await;
            detail
                .update(cx, |detail, cx| {
                    // The files were replaced meanwhile: the index names another file now.
                    let is_same_file = detail
                        .loaded_files()
                        .get(file)
                        .is_some_and(|loaded| Arc::ptr_eq(&loaded.file, &changed_file));
                    if !is_same_file {
                        return;
                    }
                    match fetched {
                        Ok(source) => {
                            let revealed = detail.context_loads_mut().finish(file);
                            let context = FileContext {
                                source: Arc::new(source),
                                revealed,
                            };
                            detail.show_context(file, context, cx);
                        }
                        Err(error) => {
                            detail.context_loads_mut().fail(file, format!("{error:#}"));
                            cx.notify();
                        }
                    }
                })
                .ok();
        });
        self.context_loads_mut().start(file, lines, task);
        cx.notify();
    }

    /// Shows file `file` with `context`'s lines revealed: rebuilds its rows
    /// and colors its grown diff. A text that does not match the patch is
    /// not used, and the gap says so.
    fn show_context(&mut self, file: usize, context: FileContext, cx: &mut Context<Self>) {
        let expanded = self
            .loaded_files()
            .get(file)
            .and_then(|loaded| loaded.with_context(context));
        let Some(expanded) = expanded else {
            self.context_loads_mut()
                .fail(file, "the file's text does not match its diff");
            cx.notify();
            return;
        };
        let Some(files) = self.loaded_files_mut() else {
            return;
        };
        let Some(slot) = files.get_mut(file) else {
            return;
        };
        *slot = expanded.clone();
        self.rebuild_file_rows(file);
        self.color_file(file, expanded, cx);
        cx.notify();
    }

    /// Works out the colors of `loaded` (file `file`'s grown diff) in the
    /// background; it shows plain until then.
    fn color_file(&mut self, file: usize, loaded: LoadedFile, cx: &mut Context<Self>) {
        let task = cx.spawn(async move |detail, cx| {
            let highlighted = cx
                .background_executor()
                .spawn(async move { HighlightedDiff::of(loaded) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    detail.apply_syntax(vec![highlighted]);
                    cx.notify();
                })
                .ok();
        });
        self.context_loads_mut().color_with(file, task);
    }
}
