use std::collections::HashMap;

use gpui::{ListAlignment, ListState, px};
use qd_cache::ReviewScope;
use qd_studio::{AgentProgress, ReaderFamiliarity};

use super::brief_key::BriefKey;
use super::brief_row::BriefRow;
use super::detail_mode::DetailMode;
use super::pending_brief::PendingBrief;
use super::review_agent_task::ReviewAgentTask;
use super::review_job_error::ReviewJobError;
use crate::brief_view::{
    BriefHandlers, BriefPageRow, BriefReport, BriefSaveStatus, BriefState, DEFAULT_DIAGRAM_WIDTH,
    PreparedBrief, ReportState,
};

/// How far beyond the visible area the brief's rows are laid out.
const BRIEF_OVERDRAW: f32 = 400.;

/// The brief of one pull request and review target, kept for the session.
struct BriefEntry {
    key: BriefKey,
    state: BriefState,
    /// The page shown of a written brief.
    page: usize,
    /// The agent run writing this brief, while one was started; dropping
    /// it (the entry removed, or a new run started) kills the agent.
    run: Option<ReviewAgentTask>,
    /// Replacement progress and errors do not hide the saved brief.
    progress: Option<AgentProgress>,
    error: Option<String>,
    /// Completed output awaiting a local save; its original receipt and metadata stay intact.
    pending_save: Option<Box<PendingBrief>>,
    saving: bool,
}

/// Which brief the shown rows belong to: the rows are laid out anew (from
/// the top) when another brief or page shows, even when its rows look the
/// same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownBrief {
    pub key: BriefKey,
    pub page: usize,
}

/// The detail's brief: whether the main column shows the summary, diff or
/// brief, the briefs written this session (by pull request and target, so
/// a layer and its combined range each have their own) with their runs,
/// and the rows of the shown brief as a virtualized list.
pub struct BriefPane {
    pub mode: DetailMode,
    entries: Vec<BriefEntry>,
    list: ListState,
    rows: Vec<BriefRow>,
    /// The brief and page `rows` show.
    shown: Option<ShownBrief>,
    /// What the brief's links and buttons do in the detail.
    handlers: BriefHandlers,
    /// How well the reader knows each repository (`owner/name`), as last
    /// picked this session; the middle answer until picked.
    familiarity: HashMap<String, ReaderFamiliarity>,
    /// Reporting the shown brief; closed again when another brief or page shows.
    report: BriefReport,
    /// The width of the brief's column, which every written brief's
    /// diagrams are laid out for.
    column_width: f32,
}

impl BriefPane {
    pub fn new(handlers: BriefHandlers) -> Self {
        Self {
            mode: DetailMode::default(),
            entries: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(BRIEF_OVERDRAW)),
            rows: Vec::new(),
            shown: None,
            handlers,
            familiarity: HashMap::new(),
            report: BriefReport::default(),
            column_width: DEFAULT_DIAGRAM_WIDTH,
        }
    }

    /// The width of the brief's column as last measured.
    pub fn column_width(&self) -> f32 {
        self.column_width
    }

    /// Lays the shown brief's diagrams out for a column `width` wide and
    /// measures the rows whose diagram changed size again, keeping the
    /// reader's place (a splice holding the top row would scroll to its
    /// start). Other briefs are fitted when they show (`set_rows`).
    pub fn set_column_width(&mut self, width: f32) {
        self.column_width = width;
        self.fit_shown_brief();
    }

    /// Fits the shown brief's diagrams to the column, measuring the rows of
    /// those that changed size again.
    fn fit_shown_brief(&mut self) {
        let Some(shown) = self.shown.clone() else {
            return;
        };
        let width = self.column_width;
        let changed = match self.entry_mut(&shown.key) {
            Some(BriefEntry {
                state: BriefState::Written(brief),
                ..
            }) => brief.fit_diagrams(width),
            Some(_) | None => return,
        };
        let top = self.list.logical_scroll_top();
        for widget in changed {
            self.remeasure(BriefRow::Page(BriefPageRow::Widget(widget)));
        }
        self.list.scroll_to(top);
    }

    /// Where reporting the shown brief stands (drawn in the bar above the
    /// rows, so no row changes size).
    pub fn report(&self) -> &ReportState {
        self.report.state()
    }

    pub fn report_mut(&mut self) -> &mut BriefReport {
        &mut self.report
    }

    /// How well the reader knows `repository`, as last picked this session.
    pub fn familiarity_for(&self, repository: &str) -> ReaderFamiliarity {
        self.familiarity
            .get(repository)
            .copied()
            .unwrap_or_default()
    }

    /// Remembers how well the reader knows `repository`.
    pub fn set_familiarity(&mut self, repository: String, familiarity: ReaderFamiliarity) {
        self.familiarity.insert(repository, familiarity);
    }

    /// Lays out `row` again (its content changed size), keeping the reader's place.
    pub fn remeasure(&mut self, row: BriefRow) {
        if let Some(index) = self.rows.iter().position(|shown| *shown == row) {
            self.list.splice(index..index + 1, 1);
        }
    }

    pub fn handlers(&self) -> &BriefHandlers {
        &self.handlers
    }

    pub fn list(&self) -> &ListState {
        &self.list
    }

    pub fn row(&self, index: usize) -> Option<BriefRow> {
        self.rows.get(index).copied()
    }

    /// The brief `key` names, if one was started.
    pub fn state_of(&self, key: &BriefKey) -> Option<&BriefState> {
        self.entry(key).map(|entry| &entry.state)
    }

    /// Installs a stored or completed brief; a separate run retains its saved predecessor.
    pub fn set_state(&mut self, key: &BriefKey, state: BriefState) {
        if let Some(entry) = self.entry_mut(key) {
            entry.state = state;
            entry.page = 0;
            entry.error = None;
            entry.pending_save = None;
            entry.saving = false;
        } else {
            self.entries.push(BriefEntry {
                key: key.clone(),
                state,
                page: 0,
                run: None,
                progress: None,
                error: None,
                pending_save: None,
                saving: false,
            });
        }
    }

    /// A new run's failure keeps the last published brief available to read.
    pub fn fail_to_start(&mut self, key: &BriefKey, message: String) {
        if let Some(entry) = self.entry_mut(key)
            && matches!(entry.state, BriefState::Written(_))
        {
            entry.error = Some(message);
        } else {
            self.set_state(key, BriefState::Failed(message.into()));
        }
    }

    /// The current generation identity; used to reject late progress and completion.
    pub fn run_of(&self, key: &BriefKey) -> Option<&ReviewAgentTask> {
        self.entry(key)?.run.as_ref()
    }

    /// The latest progress of the current generation; absent until its first event.
    pub fn run_progress_of(&self, key: &BriefKey) -> Option<AgentProgress> {
        self.entry(key)?.progress
    }

    /// Why a replacement failed, while its previous brief is still shown.
    pub fn error_of(&self, key: &BriefKey) -> Option<&str> {
        self.entry(key)?.error.as_deref()
    }

    /// Distinguishes saving completed output from asking Claude for a new brief.
    pub fn save_status_of(&self, key: &BriefKey) -> BriefSaveStatus {
        match self.entry(key) {
            Some(entry) if entry.saving => BriefSaveStatus::Saving,
            Some(entry) if entry.pending_save.is_some() => BriefSaveStatus::Pending,
            _ => BriefSaveStatus::None,
        }
    }

    /// The immutable completed draft, checked against the currently loaded revision before retry.
    pub(super) fn pending_save_of(&self, key: &BriefKey) -> Option<&PendingBrief> {
        self.entry(key)?.pending_save.as_deref()
    }

    /// Transfers the completed draft to one background save attempt.
    pub(super) fn take_pending_save(&mut self, key: &BriefKey) -> Option<Box<PendingBrief>> {
        self.entry_mut(key)?.pending_save.take()
    }

    /// The page shown of the brief `key` names.
    pub fn page_of(&self, key: &BriefKey) -> usize {
        self.entry(key).map_or(0, |entry| entry.page)
    }

    /// Shows `page` of the brief `key` names.
    pub fn set_page(&mut self, key: &BriefKey, page: usize) {
        if let Some(entry) = self.entry_mut(key) {
            entry.page = page;
        }
    }

    fn entry(&self, key: &BriefKey) -> Option<&BriefEntry> {
        self.entries.iter().find(|entry| entry.key == *key)
    }

    fn entry_mut(&mut self, key: &BriefKey) -> Option<&mut BriefEntry> {
        self.entries.iter_mut().find(|entry| entry.key == *key)
    }

    /// Starts a generation without removing a previously published brief.
    pub fn start_run(&mut self, key: &BriefKey, run: ReviewAgentTask) {
        if self.entry(key).is_none() {
            self.set_state(key, BriefState::Writing(None));
        }
        if let Some(entry) = self.entry_mut(key) {
            if !matches!(entry.state, BriefState::Written(_)) {
                entry.state = BriefState::Writing(None);
            }
            entry.run = Some(run);
            entry.progress = None;
            entry.error = None;
            entry.pending_save = None;
            entry.saving = false;
        }
    }

    /// Saves a retained draft under its original generation without invoking Claude.
    pub fn start_save(&mut self, key: &BriefKey, run: ReviewAgentTask) {
        self.start_run(key, run);
        if let Some(entry) = self.entry_mut(key) {
            entry.saving = true;
        }
    }

    /// Applies progress only to the generation still owned by this entry.
    pub fn update_run_progress(&mut self, key: &BriefKey, run_id: &str, progress: AgentProgress) {
        if let Some(entry) = self.entry_mut(key)
            && entry.run.as_ref().is_some_and(|run| run.run_id == run_id)
        {
            entry.progress = Some(progress);
            if matches!(entry.state, BriefState::Writing(_)) {
                entry.state = BriefState::Writing(Some(progress));
            }
        }
    }

    /// Completes only the current generation, preserving prior content on failure.
    pub fn finish_run(
        &mut self,
        key: &BriefKey,
        run_id: &str,
        outcome: Result<PreparedBrief, ReviewJobError<PendingBrief>>,
    ) -> bool {
        let replaces_shown = self.shown.as_ref().is_some_and(|shown| shown.key == *key);
        let Some(entry) = self.entry_mut(key) else {
            return false;
        };
        if entry.run.as_ref().is_none_or(|run| run.run_id != run_id) {
            return false;
        }
        entry.run = None;
        entry.progress = None;
        entry.saving = false;
        entry.pending_save = None;
        match outcome {
            Ok(brief) => {
                entry.state = BriefState::Written(Box::new(brief));
                entry.page = 0;
                entry.error = None;
                // Identical row positions can still contain a wholly new artifact.
                // Force new measurements and close the previous report on the next refresh.
                if replaces_shown {
                    self.shown = None;
                }
            }
            Err(error) => {
                let message = match error {
                    ReviewJobError::Failed(message) => message,
                    ReviewJobError::SaveFailed { draft, message } => {
                        entry.pending_save = Some(draft);
                        message
                    }
                };
                if matches!(entry.state, BriefState::Written(_)) {
                    entry.error = Some(message);
                } else {
                    entry.state = BriefState::Failed(message.into());
                }
            }
        }
        true
    }

    /// Cancels until publication starts; a written brief and its page remain intact.
    pub fn cancel_run(&mut self, key: &BriefKey) {
        if let Some(entry) = self.entry_mut(key)
            && entry
                .run
                .as_ref()
                .map_or(entry.pending_save.is_some(), |run| run.gate.cancel())
        {
            entry.run = None;
            entry.progress = None;
            entry.error = None;
            entry.pending_save = None;
            entry.saving = false;
            if !matches!(entry.state, BriefState::Written(_)) {
                self.entries.retain(|entry| entry.key != *key);
            }
        }
    }

    /// Refresh invalidates drafts of this scope when either captured commit changes.
    pub fn invalidate_runs(
        &mut self,
        scope: &ReviewScope,
        head_sha: Option<&str>,
        base_sha: Option<&str>,
    ) {
        for entry in &mut self.entries {
            let invalidated_run = entry.run.as_ref().is_some_and(|run| {
                &run.scope == scope
                    && (run.head_sha.as_deref() != head_sha || run.base_sha.as_deref() != base_sha)
                    && run.gate.invalidate()
            });
            let invalidated_save = entry.pending_save.as_ref().is_some_and(|draft| {
                draft.scope() == scope
                    && (draft.head_sha() != head_sha || draft.base_sha() != base_sha)
            });
            if invalidated_run || invalidated_save {
                entry.run = None;
                entry.progress = None;
                entry.pending_save = None;
                entry.saving = false;
                let message =
                    "The diff changed while writing the brief. Write it again for this revision."
                        .to_owned();
                if matches!(entry.state, BriefState::Written(_)) {
                    entry.error = Some(message);
                } else {
                    entry.state = BriefState::Failed(message.into());
                }
            }
        }
    }

    /// Shows `rows` of the brief and page `shown`; the list is reset (to its
    /// top, the report panel closed, the brief's diagrams fitted to the
    /// column) when either changed, so a refresh that
    /// changes nothing keeps the reader's place while another brief or page
    /// starts at the top.
    pub fn set_rows(&mut self, shown: Option<ShownBrief>, rows: Vec<BriefRow>) {
        if rows != self.rows || shown != self.shown {
            self.list.reset(rows.len());
            self.rows = rows;
            self.shown = shown;
            self.report.close();
            self.fit_shown_brief();
        }
    }
}
