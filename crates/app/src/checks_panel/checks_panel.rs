use std::time::Duration;

use gpui::{Context, ListAlignment, ListState, SharedString, Task, px};

use super::check_row::{CheckRow, SkippedChecks};
use super::checks_target::ChecksTarget;
use super::shown_checks::ShownChecks;
use super::unfolded_checks::UnfoldedChecks;
use crate::app_services::AppServices;
use crate::display_text::unix_now;

/// How often the checks are read again while some are pending and the tab shows.
const REFRESH_INTERVAL: Duration = Duration::from_secs(30);
/// How far beyond the visible area rows are laid out.
const LIST_OVERDRAW: f32 = 300.;

/// The Checks tab of the side panel: the shown pull request's checks on
/// its head commit, read from GitHub when the tab shows (one GraphQL
/// request per 100 checks) and again every 30 s while some are pending and
/// the tab stays open. Nothing is cached: checks change by the minute.
pub struct ChecksPanel {
    services: AppServices,
    target: Option<ChecksTarget>,
    is_visible: bool,
    /// The checks last read for the target, kept while they are read again.
    shown: Option<ShownChecks>,
    is_loading: bool,
    /// Why the last read failed, if it did.
    error: Option<SharedString>,
    unfolded: UnfoldedChecks,
    skipped: SkippedChecks,
    rows: Vec<CheckRow>,
    list: ListState,
    /// The read in flight; replacing it cancels the previous one.
    load_task: Option<Task<()>>,
    /// The wait before the next read; dropped when the tab hides.
    refresh_task: Option<Task<()>>,
}

impl ChecksPanel {
    /// A panel showing nothing until a pull request is shown.
    pub fn new(services: AppServices) -> Self {
        Self {
            services,
            target: None,
            is_visible: false,
            shown: None,
            is_loading: false,
            error: None,
            unfolded: UnfoldedChecks::default(),
            skipped: SkippedChecks::default(),
            rows: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(LIST_OVERDRAW)),
            load_task: None,
            refresh_task: None,
        }
    }

    pub(super) fn shown(&self) -> Option<&ShownChecks> {
        self.shown.as_ref()
    }

    pub(super) fn is_loading(&self) -> bool {
        self.is_loading
    }

    pub(super) fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    pub(super) fn rows(&self) -> &[CheckRow] {
        &self.rows
    }

    pub(super) fn list(&self) -> &ListState {
        &self.list
    }

    pub(super) fn unfolded(&self) -> &UnfoldedChecks {
        &self.unfolded
    }

    pub(super) fn skipped(&self) -> SkippedChecks {
        self.skipped
    }

    /// Why no checks can be read at all: no GitHub connection.
    pub(super) fn connection_error(&self) -> Option<&str> {
        self.services.github.as_ref().err().map(String::as_str)
    }

    /// Follows the pull request the detail shows. Another pull request
    /// starts empty and folded; a new version of the same one is read again
    /// (new commits bring new checks), keeping the old ones shown meanwhile.
    pub fn show_pull_request(&mut self, target: Option<ChecksTarget>, cx: &mut Context<Self>) {
        if self.target == target {
            return;
        }
        let is_same_pull_request = match (&self.target, &target) {
            (Some(shown), Some(next)) => shown.is_same_pull_request(next),
            _ => false,
        };
        self.target = target;
        if !is_same_pull_request {
            self.shown = None;
            self.error = None;
            self.unfolded.clear();
            self.load_task = None;
            self.is_loading = false;
            self.rebuild_rows();
        }
        if self.is_visible {
            self.load(cx);
        }
        cx.notify();
    }

    /// The tab is shown: reads the checks unless a recent read is shown,
    /// and keeps them current while some are pending.
    pub fn show(&mut self, cx: &mut Context<Self>) {
        if self.is_visible {
            return;
        }
        self.is_visible = true;
        let is_recent = self
            .shown
            .as_ref()
            .is_some_and(|shown| unix_now() - shown.read_at < interval_seconds());
        if is_recent {
            self.schedule_refresh(cx);
        } else {
            self.load(cx);
        }
    }

    /// The tab is hidden: stops reading the checks again.
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.refresh_task = None;
    }

    /// Reads the target's checks from GitHub now.
    pub(super) fn load(&mut self, cx: &mut Context<Self>) {
        let (Some(target), Ok(client)) = (self.target.clone(), self.services.github.clone()) else {
            return;
        };
        self.is_loading = true;
        self.refresh_task = None;
        self.load_task = Some(cx.spawn(async move |panel, cx| {
            let repo = target.repo.clone();
            let checks = cx
                .background_executor()
                .spawn(async move {
                    client.pull_request_checks(&repo, target.number, &target.layers_below)
                })
                .await;
            panel
                .update(cx, |panel, cx| {
                    panel.is_loading = false;
                    match checks {
                        Ok(checks) => {
                            panel.error = None;
                            panel.shown = Some(ShownChecks::arrange(checks));
                        }
                        Err(error) => panel.error = Some(format!("{error:#}").into()),
                    }
                    panel.rebuild_rows();
                    panel.schedule_refresh(cx);
                    cx.notify();
                })
                .ok();
        }));
        cx.notify();
    }

    /// Folds or unfolds a workflow, or a group in one.
    pub(super) fn toggle_fold(
        &mut self,
        workflow: &str,
        group: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.unfolded.toggle(workflow, group);
        self.rebuild_rows();
        cx.notify();
    }

    /// Shows or hides the checks that did not run.
    pub(super) fn toggle_skipped(&mut self, cx: &mut Context<Self>) {
        self.skipped = match self.skipped {
            SkippedChecks::Hidden => SkippedChecks::Shown,
            SkippedChecks::Shown => SkippedChecks::Hidden,
        };
        self.rebuild_rows();
        cx.notify();
    }

    /// Reads again after the interval while the tab shows and some checks
    /// are pending; otherwise nothing is scheduled.
    fn schedule_refresh(&mut self, cx: &mut Context<Self>) {
        let is_pending = self
            .shown
            .as_ref()
            .is_some_and(ShownChecks::has_pending_checks);
        if !self.is_visible || !is_pending {
            self.refresh_task = None;
            return;
        }
        self.refresh_task = Some(cx.spawn(async move |panel, cx| {
            cx.background_executor().timer(REFRESH_INTERVAL).await;
            panel.update(cx, Self::load).ok();
        }));
    }

    /// Lays the rows out again from the shown checks, folds and skip choice.
    fn rebuild_rows(&mut self) {
        self.rows = match self.shown.as_ref().and_then(|shown| shown.content.tree()) {
            Some(tree) => CheckRow::build_all(tree, &self.unfolded, self.skipped),
            None => Vec::new(),
        };
        self.list.reset(self.rows.len());
    }
}

/// [`REFRESH_INTERVAL`] in seconds, to compare with Unix times.
fn interval_seconds() -> i64 {
    i64::try_from(REFRESH_INTERVAL.as_secs()).unwrap_or(i64::MAX)
}
