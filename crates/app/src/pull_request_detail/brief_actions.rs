use std::fmt::Write as _;

use futures::StreamExt as _;
use futures::channel::mpsc;
use std::rc::Rc;

use gpui::Context;
use qd_file_tree::FileKind;
use qd_github::{PullRequest, PullRequestStack};
use qd_studio::{
    CodeAccess, CodeAnchor, CodeSide, FileListing, ReaderFamiliarity, RequestLayer, StudioRequest,
    StudioRequestFile, find_claude,
};

use super::brief_job::BriefJob;
use super::brief_pane::ShownBrief;
use super::brief_row::BriefRow;
use super::detail_event::DetailEvent;
use super::detail_mode::DetailMode;
use super::detail_row::DetailRow;
use super::file_lookup::file_index_of;
use super::loaded_file::LoadedFile;
use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use super::run_cancel::RunCancel;
use crate::brief_view::{
    BriefHandlers, BriefPageRow, BriefReport, BriefState, PreparedBrief, ReportHost, view_action,
};
use crate::ui_state::UiStateRecorder;

/// The report panel's text field while empty.
const REPORT_PLACEHOLDER: &str = "What is wrong or weird: a part that misleads, is missing, is too long, a widget that looks off…";
/// Shown when no `claude` executable is found.
const CLAUDE_MISSING: &str = "Claude Code was not found. Install it (claude.ai/code) so `claude` is on PATH, in ~/.local/bin or in Homebrew's folder, and log in once in a terminal.";

impl PullRequestDetail {
    /// The shown pull request's brief at the shown target, if one was started.
    pub(super) fn current_brief(&self) -> Option<&BriefState> {
        let pull_request = self.pull_request()?;
        self.brief().state_of(pull_request.id, self.target())
    }

    /// Shows the diff or the brief in the main column.
    pub(super) fn set_detail_mode(&mut self, mode: DetailMode, cx: &mut Context<Self>) {
        self.brief_mut().mode = mode;
        self.refresh_brief_rows();
        cx.notify();
    }

    /// The page shown of the shown brief.
    pub(super) fn current_brief_page(&self) -> usize {
        self.pull_request().map_or(0, |pull_request| {
            self.brief().page_of(pull_request.id, self.target())
        })
    }

    /// Shows `brief`, read from storage, as the shown pull request's brief
    /// at the shown target, unless this session already has one there (a
    /// run in flight, or one written since).
    pub(super) fn show_stored_brief(&mut self, brief: PreparedBrief) {
        let Some(pull_request_id) = self.pull_request().map(|pull_request| pull_request.id) else {
            return;
        };
        let target = self.target().clone();
        if self.brief().state_of(pull_request_id, &target).is_none() {
            self.brief_mut().set_state(
                pull_request_id,
                &target,
                BriefState::Written(Box::new(brief)),
            );
        }
    }

    /// Lays the shown brief's diagrams out for a column `width` wide,
    /// measured by the brief area's probe when the reader resizes the pane.
    pub(super) fn set_brief_column_width(&mut self, width: f32, cx: &mut Context<Self>) {
        self.brief_mut().set_column_width(width);
        cx.notify();
    }

    /// Lays out the brief's rows for the shown pull request and target:
    /// the status row until a brief is written, then the shown page.
    pub(super) fn refresh_brief_rows(&mut self) {
        let shown = self.pull_request().map(|pull_request| ShownBrief {
            pull_request_id: pull_request.id,
            target: self.target().clone(),
            page: self.current_brief_page(),
        });
        let rows = match self.current_brief() {
            Some(BriefState::Written(brief)) => {
                BriefPageRow::rows_of(brief, self.current_brief_page())
                    .into_iter()
                    .map(BriefRow::Page)
                    .collect()
            }
            Some(BriefState::Writing(_) | BriefState::Failed(_)) | None => vec![BriefRow::Status],
        };
        self.brief_mut().set_rows(shown, rows);
    }

    /// How well the reader knows the shown pull request's repository, as
    /// last picked this session.
    pub(super) fn current_familiarity(&self) -> ReaderFamiliarity {
        self.pull_request()
            .map_or_else(ReaderFamiliarity::default, |pull_request| {
                self.brief().familiarity_for(&pull_request.repo.full_name())
            })
    }

    /// Remembers how well the reader knows the shown pull request's
    /// repository; the status card or footer showing the picker is laid out
    /// again, since the line explaining the choice changes.
    pub(super) fn choose_familiarity(
        &mut self,
        familiarity: ReaderFamiliarity,
        cx: &mut Context<Self>,
    ) {
        let Some(repository) = self
            .pull_request()
            .map(|pull_request| pull_request.repo.full_name())
        else {
            return;
        };
        self.brief_mut().set_familiarity(repository, familiarity);
        self.brief_mut().remeasure(BriefRow::Status);
        self.brief_mut()
            .remeasure(BriefRow::Page(BriefPageRow::Footer));
        cx.notify();
    }

    /// Writes a report on the shown brief with the words in the report
    /// panel ([`ReportHost::save_report`]).
    fn save_shown_brief_report(&mut self, cx: &mut Context<Self>) {
        if let Some(BriefState::Written(brief)) = self.current_brief() {
            let record = brief.record.clone();
            self.save_report(record, app_version(), cx);
        }
    }

    /// Shows page `page` of the shown brief, from its top.
    pub(super) fn show_brief_page(&mut self, page: usize, cx: &mut Context<Self>) {
        let Some(pull_request_id) = self.pull_request().map(|pull_request| pull_request.id) else {
            return;
        };
        let target = self.target().clone();
        self.brief_mut().set_page(pull_request_id, &target, page);
        self.refresh_brief_rows();
        cx.notify();
    }

    /// Starts Claude Code on the shown pull request's loaded files, in the
    /// background. Progress shows in the brief's status row; the brief
    /// replaces it when written. A run already in flight is cancelled.
    pub(super) fn write_brief(&mut self, cx: &mut Context<Self>) {
        // The pull request is cloned: the background run checks out its commits.
        let Some(pull_request) = self.pull_request().cloned() else {
            return;
        };
        if self.loaded_files().is_empty() {
            return;
        }
        let pull_request_id = pull_request.id;
        let target = self.target().clone();
        let reader = self.current_familiarity();
        let request = studio_request(
            &pull_request,
            &target,
            self.stack(),
            self.loaded_files(),
            reader,
        );

        // Without the CLI there is nothing to run; say how to get it.
        let Some(program) = find_claude() else {
            self.brief_mut().set_state(
                pull_request_id,
                &target,
                BriefState::Failed(CLAUDE_MISSING.into()),
            );
            self.refresh_brief_rows();
            cx.notify();
            return;
        };
        self.brief_mut()
            .set_state(pull_request_id, &target, BriefState::Writing(None));
        self.refresh_brief_rows();
        cx.notify();

        let (cancel, cancel_flag) = RunCancel::new();
        let job = BriefJob {
            services: self.services().clone(),
            pull_request,
            target: target.clone(),
            head_sha: self.shown_head_sha().map(str::to_owned),
            program,
            request,
            files: self.loaded_files().to_vec(),
            store: cx.global::<UiStateRecorder>().shared_store(),
        };
        let run_target = target.clone();
        let task = cx.spawn(async move |detail, cx| {
            // Write it on the background executor, reporting progress through
            // a channel that ends with the run.
            let (progress_sender, mut progress) = mpsc::unbounded();
            let run = cx.background_executor().spawn(async move {
                let mut report = |step| {
                    progress_sender.unbounded_send(step).ok();
                };
                job.run(&cancel_flag, &mut report)
            });

            // Show each step; a closed view ends the wait.
            while let Some(step) = progress.next().await {
                let shown = detail.update(cx, |detail, cx| {
                    detail.set_brief_state(
                        pull_request_id,
                        &target,
                        BriefState::Writing(Some(step)),
                    );
                    cx.notify();
                });
                if shown.is_err() {
                    return;
                }
            }
            let outcome = run.await;

            // Show the brief, or why there is none.
            detail
                .update(cx, |detail, cx| {
                    let state = match outcome {
                        Ok(brief) => {
                            cx.emit(DetailEvent::BriefStored);
                            BriefState::Written(Box::new(brief))
                        }
                        Err(message) => BriefState::Failed(message.into()),
                    };
                    detail.set_brief_state(pull_request_id, &target, state);
                    cx.notify();
                })
                .ok();
        });
        self.brief_mut()
            .start_run(pull_request_id, &run_target, task, cancel);
    }

    /// Stops the shown brief's run (killing the agent); the brief it was
    /// replacing shows again, if there was one.
    pub(super) fn cancel_brief(&mut self, cx: &mut Context<Self>) {
        if let Some(pull_request_id) = self.pull_request().map(|pull_request| pull_request.id) {
            let target = self.target().clone();
            self.brief_mut().cancel_run(pull_request_id, &target);
        }
        self.refresh_brief_rows();
        cx.notify();
    }

    /// Records a brief's new state, laying out the rows again when it is the shown one.
    fn set_brief_state(&mut self, pull_request_id: u64, target: &ReviewTarget, state: BriefState) {
        self.brief_mut().set_state(pull_request_id, target, state);
        self.refresh_brief_rows();
    }

    /// Shows file `file`'s header in the diff (a file clicked in the file
    /// tree), switching from the brief to the diff first.
    pub(super) fn show_file_in_diff(&mut self, file: usize, cx: &mut Context<Self>) {
        if self.brief().mode == DetailMode::Brief {
            self.set_detail_mode(DetailMode::Diff, cx);
        }
        self.reveal_file(file, cx);
    }

    /// Switches to the diff with `anchor`'s first line at the top: the file
    /// is expanded when collapsed; when the line has no row (a filter hides
    /// the file) the diff shows the file's header, or stays where it was.
    pub(super) fn show_anchor_in_diff(&mut self, anchor: &CodeAnchor, cx: &mut Context<Self>) {
        self.set_detail_mode(DetailMode::Diff, cx);
        let Some(file) = file_index_of(self.loaded_files(), &anchor.path) else {
            return;
        };
        // Expand the file, so its lines have rows.
        let loaded = &self.loaded_files()[file];
        if self.file_review().state_of(loaded).is_collapsed {
            self.toggle_file_collapsed(file, cx);
        }
        match self.row_of_line(file, anchor.side, anchor.first_line) {
            Some(row) => {
                self.scroll_to_row(row);
                cx.notify();
            }
            None => self.reveal_file(file, cx),
        }
    }

    /// The diff row showing line `number` on `side` of file `file`.
    fn row_of_line(&self, file: usize, side: CodeSide, number: u32) -> Option<usize> {
        let range = self.rows().file_range(file)?;
        let diff = &self.loaded_files().get(file)?.diff;
        let number_at = |hunk: usize, line: usize| {
            let line = diff.hunks().get(hunk)?.lines.get(line)?;
            match side {
                CodeSide::Base => line.old_number,
                CodeSide::Head => line.new_number,
            }
        };
        let start = range.start;
        self.rows()
            .rows_in(range)
            .iter()
            .position(|row| match *row {
                DetailRow::UnifiedLine { hunk, line, .. } => number_at(hunk, line) == Some(number),
                DetailRow::SplitLine { hunk, row, .. } => [row.old, row.new]
                    .into_iter()
                    .flatten()
                    .any(|line| number_at(hunk, line) == Some(number)),
                _ => false,
            })
            .map(|offset| start + offset)
    }
}

/// What the brief's links and buttons do in the detail: an anchor opens the
/// diff there, the buttons run or cancel the agent, tabs switch pages. The handlers hold a weak handle, so a closed view
/// ignores them.
pub(super) fn detail_brief_handlers(cx: &mut Context<PullRequestDetail>) -> BriefHandlers {
    let detail = cx.entity().downgrade();
    BriefHandlers {
        show_anchor: Rc::new({
            let detail = detail.clone();
            move |anchor, _, cx| {
                detail
                    .update(cx, |detail, cx| detail.show_anchor_in_diff(anchor, cx))
                    .ok();
            }
        }),
        write: view_action(&detail, |detail, _, cx| detail.write_brief(cx)),
        cancel: view_action(&detail, |detail, _, cx| detail.cancel_brief(cx)),
        show_page: Rc::new({
            let detail = detail.clone();
            move |page, _, cx| {
                detail
                    .update(cx, |detail, cx| detail.show_brief_page(page, cx))
                    .ok();
            }
        }),
        choose_reader: Rc::new({
            let detail = detail.clone();
            move |familiarity, _, cx| {
                detail
                    .update(cx, |detail, cx| detail.choose_familiarity(familiarity, cx))
                    .ok();
            }
        }),
        open_report: view_action(&detail, |detail, window, cx| {
            detail.open_report(REPORT_PLACEHOLDER, window, cx);
        }),
        save_report: view_action(&detail, |detail, _, cx| detail.save_shown_brief_report(cx)),
        close_report: view_action(&detail, |detail, _, cx| detail.close_report(cx)),
        copy_report_path: view_action(&detail, |detail, _, cx| detail.copy_report_path(cx)),
    }
}

/// The report panel lays itself out in the bar above the brief's rows, so
/// the detail only hands out the pane's report.
impl ReportHost for PullRequestDetail {
    fn report_mut(&mut self) -> &mut BriefReport {
        self.brief_mut().report_mut()
    }
}

/// This build's version for a report: the package version, marked as a
/// development build when it is one.
fn app_version() -> String {
    let version = env!("CARGO_PKG_VERSION");
    if cfg!(debug_assertions) {
        format!("{version} (development build)")
    } else {
        version.to_owned()
    }
}

/// What the agent is told about the shown pull request at `target`, with
/// each loaded file's diff or why it is left out, for a combined range the
/// stack layers it spans (from `stack`), bottom first, and how well the
/// `reader` knows the area. The code access is set once the checkout ran.
fn studio_request(
    pull_request: &PullRequest,
    target: &ReviewTarget,
    stack: Option<&PullRequestStack>,
    files: &[LoadedFile],
    reader: ReaderFamiliarity,
) -> StudioRequest {
    let layers = match (target, stack) {
        (ReviewTarget::Combined(range), Some(stack)) => stack
            .layers
            .get(range.first_index..=range.last_index)
            .unwrap_or_default()
            .iter()
            .map(|layer| RequestLayer {
                number: layer.number,
                title: layer.title.clone(),
                // The shown pull request's own; the others' are read with the run.
                description: (layer.number == pull_request.number)
                    .then(|| pull_request.body.clone())
                    .flatten(),
            })
            .collect(),
        _ => Vec::new(),
    };
    let subject = match target {
        ReviewTarget::Combined(range) => format!(
            "Stack layers #{} to #{} combined ({} layers), up to pull request #{}: {}",
            range.first_number,
            range.last_number,
            range.layer_count(),
            pull_request.number,
            pull_request.title
        ),
        ReviewTarget::Layer | ReviewTarget::AwaitingStack => {
            format!(
                "Pull request #{}: {}",
                pull_request.number, pull_request.title
            )
        }
    };
    StudioRequest {
        repository: pull_request.repo.full_name(),
        subject,
        author: pull_request.author.login.clone(),
        description: pull_request.body.clone().unwrap_or_default(),
        files: files
            .iter()
            .map(|loaded| StudioRequestFile {
                path: loaded.file.filename.clone(),
                previous_path: loaded.file.previous_filename.clone(),
                status: loaded.file.status.clone(),
                additions: loaded.file.additions,
                deletions: loaded.file.deletions,
                listing: listing_of(loaded),
            })
            .collect(),
        code: CodeAccess::DiffOnly("not prepared yet".into()),
        reader,
        layers,
    }
}

/// The loaded files' patches as one unified diff, for the workspace's
/// `change.diff`: every file (generated and moved ones too), with `---` /
/// `+++` headers (`/dev/null` for an added or removed side, the previous
/// path for a rename) so the agent can see what each file was before; a
/// file without a patch is named with why.
pub(super) fn change_diff_of(files: &[LoadedFile]) -> String {
    let mut diff = String::new();
    for loaded in files {
        let file = &loaded.file;
        let old_path = file.previous_filename.as_deref().unwrap_or(&file.filename);
        let (old_side, new_side) = match file.status.as_str() {
            "added" => ("/dev/null".to_owned(), format!("b/{}", file.filename)),
            "removed" => (format!("a/{old_path}"), "/dev/null".to_owned()),
            _ => (format!("a/{old_path}"), format!("b/{}", file.filename)),
        };
        let _ = writeln!(diff, "diff --git a/{old_path} b/{}", file.filename);
        match &file.patch {
            Some(patch) => {
                let _ = writeln!(diff, "--- {old_side}\n+++ {new_side}");
                diff.push_str(patch);
                if !patch.ends_with('\n') {
                    diff.push('\n');
                }
            }
            None => {
                let _ = writeln!(diff, "(no patch: binary, or too large for GitHub to show)");
            }
        }
    }
    diff
}

/// The file's diff for the agent, or why it is left out: generated and
/// unchanged moved files say little, and a file without a patch has none.
fn listing_of(loaded: &LoadedFile) -> FileListing {
    match (loaded.facets.kind, loaded.file.patch.is_some()) {
        (FileKind::Generated, _) => FileListing::LeftOut("generated".into()),
        (FileKind::Moved, _) => FileListing::LeftOut("moved without changes".into()),
        (FileKind::Code | FileKind::Tests, false) => {
            FileListing::LeftOut("no patch (binary, or too large for GitHub to show)".into())
        }
        (FileKind::Code | FileKind::Tests, true) => FileListing::Diff(loaded.diff.clone()),
    }
}

#[cfg(test)]
mod tests {
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;

    use super::*;

    fn loaded(
        filename: &str,
        previous: Option<&str>,
        status: &str,
        patch: Option<&str>,
    ) -> LoadedFile {
        LoadedFile::build(
            PullRequestFile {
                filename: filename.into(),
                previous_filename: previous.map(Into::into),
                status: status.into(),
                additions: 1,
                deletions: 1,
                patch: patch.map(Into::into),
                sha: None,
            },
            DiffOptions::default(),
        )
    }

    /// The workspace's diff holds every file with headers that say what it
    /// was before: added from nothing, renamed from its old path, and a
    /// file without a patch named.
    #[test]
    fn writes_every_file_into_the_change_diff() {
        let diff = change_diff_of(&[
            loaded(
                "src/new.rs",
                None,
                "added",
                Some("@@ -0,0 +1 @@\n+fn new() {}"),
            ),
            loaded(
                "src/lib.rs",
                Some("src/old.rs"),
                "renamed",
                Some("@@ -1 +1 @@\n-a\n+b\n"),
            ),
            loaded("logo.png", None, "modified", None),
        ]);
        assert!(
            diff.contains("--- /dev/null\n+++ b/src/new.rs\n@@ -0,0 +1 @@\n+fn new() {}\n"),
            "{diff}"
        );
        assert!(
            diff.contains(
                "diff --git a/src/old.rs b/src/lib.rs\n--- a/src/old.rs\n+++ b/src/lib.rs\n"
            ),
            "{diff}"
        );
        assert!(
            diff.contains("diff --git a/logo.png b/logo.png\n(no patch"),
            "{diff}"
        );
    }
}
