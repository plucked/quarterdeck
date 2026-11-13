use std::collections::HashMap;

use gpui::{ListAlignment, ListState, Task, px};
use qd_studio::ReaderFamiliarity;

use super::brief_key::BriefKey;
use super::brief_row::BriefRow;
use super::detail_mode::DetailMode;
use super::run_cancel::RunCancel;
use crate::brief_view::{
    BriefHandlers, BriefPageRow, BriefReport, BriefState, DEFAULT_DIAGRAM_WIDTH, PreparedBrief,
    ReportState,
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
    run: Option<BriefRun>,
    /// The written brief a run in flight replaces, shown again when that
    /// run is cancelled.
    replaced: Option<Box<PreparedBrief>>,
}

/// An agent run: its task, and the guard that kills the agent process when
/// the run is replaced or the view closes.
struct BriefRun {
    _task: Task<()>,
    _cancel: RunCancel,
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

    /// Records the brief `key` names, replacing the state before (its run
    /// goes on); it opens on its first page, its diagrams fitted to the
    /// column when it shows. A written brief that a run starts to replace is
    /// kept, to show again when the run is cancelled.
    pub fn set_state(&mut self, key: &BriefKey, state: BriefState) {
        let Some(entry) = self.entry_mut(key) else {
            self.entries.push(BriefEntry {
                key: key.clone(),
                state,
                page: 0,
                run: None,
                replaced: None,
            });
            return;
        };

        // Writing keeps the brief it replaces until the run ends.
        let earlier = std::mem::replace(&mut entry.state, state);
        entry.replaced = match (earlier, &entry.state) {
            (BriefState::Written(brief), BriefState::Writing(_)) => Some(brief),
            (_, BriefState::Writing(_)) => entry.replaced.take(),
            (_, BriefState::Written(_) | BriefState::Failed(_)) => None,
        };
        entry.page = 0;
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

    /// Keeps `task` as the run writing the brief `key` names (recorded as
    /// writing before), cancelling an earlier run of that brief; runs of
    /// other briefs go on.
    pub fn start_run(&mut self, key: &BriefKey, task: Task<()>, cancel: RunCancel) {
        if let Some(entry) = self.entry_mut(key) {
            entry.run = Some(BriefRun {
                _task: task,
                _cancel: cancel,
            });
        }
    }

    /// Stops the run writing the brief `key` names (killing its agent) and
    /// shows the brief it was replacing again, or forgets the brief when
    /// there was none.
    pub fn cancel_run(&mut self, key: &BriefKey) {
        let replaced = self.entry_mut(key).and_then(|entry| entry.replaced.take());
        self.remove(key);
        if let Some(brief) = replaced {
            self.set_state(key, BriefState::Written(brief));
        }
    }

    /// Forgets the brief `key` names, killing its run.
    fn remove(&mut self, key: &BriefKey) {
        self.entries.retain(|entry| entry.key != *key);
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
