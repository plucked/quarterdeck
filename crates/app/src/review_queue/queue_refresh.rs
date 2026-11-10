use std::time::{Duration, Instant};

use gpui::{Context, Window};
use qd_cache::PullRequestListKey;
use qd_github::GitHubError;

use super::ReviewQueue;
use super::activation_refresh::ActivationRefresh;
use super::fetch_queue_lists::{
    FetchedOthers, OwnLists, RefreshInput, fetch_others, fetch_own_lists, fill_stacks,
    known_details,
};
use super::hidden_branches::without_hidden_branches;
use super::queue_lists::QueueLists;
use super::refresh_status::RefreshStatus;
use super::restore_phase::RestorePhase;
use super::selection_after_refresh::SelectionAfterRefresh;
use super::selection_origin::SelectionOrigin;
use crate::app_settings::AppSettings;

impl ReviewQueue {
    /// Fetches the lists and the stacks of their pull requests from GitHub,
    /// caches and shows them (the viewer's own first, then others' in the
    /// watched repositories), removes cached data unused for longer than the
    /// cache TTL, then arms the next timed refresh. One refresh runs at a
    /// time: asked for during one, it runs once that one finishes.
    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        // A refresh in flight finishes first: two would send their requests at once.
        if matches!(self.refresh_status, RefreshStatus::Refreshing) {
            self.is_refresh_requested = true;
            return;
        }

        // Any refresh moves the timed one: it is re-armed when this one finishes.
        self.auto_refresh_task = None;

        // Without a client there is nothing to refresh; say why.
        let client = match &self.services.github {
            Ok(client) => client.clone(),
            Err(reason) => {
                self.refresh_status = RefreshStatus::Failed(reason.clone().into());
                cx.notify();
                return;
            }
        };
        let cache = self.services.cache.clone();
        let ttl = AppSettings::cache_ttl(cx).duration();
        let input = RefreshInput {
            watched: self.others_tab().watched().to_vec(),
            known_details: known_details(&self.fetched),
            known_others: self.fetched.others.clone(),
            others_synced_until: self.others_tab().synced_until().clone(),
        };

        self.refresh_status = RefreshStatus::Refreshing;
        cx.notify();
        self.refresh_task = Some(cx.spawn(async move |queue, cx| {
            // The viewer's own lists and every stack first, off the main thread
            // (every call blocks), so they show without waiting for others'.
            let own_client = client.clone();
            let own_cache = cache.clone();
            let watched = input.watched.clone();
            let known_details = input.known_details;
            let own =
                cx.background_executor()
                    .spawn(async move {
                        fetch_own_lists(&own_client, &own_cache, &watched, known_details)
                    })
                    .await;
            let own = match own {
                Ok(own) => own,
                Err(error) => {
                    queue
                        .update(cx, |queue, cx| queue.finish_refresh(Some(&error), None, cx))
                        .ok();
                    return;
                }
            };
            let details = own.details.clone();
            if queue
                .update(cx, |queue, cx| queue.show_own_lists(own, cx))
                .is_err()
            {
                return;
            }

            // Then others' pull requests, cached; then drop what has not been used within the TTL.
            let others = cx
                .background_executor()
                .spawn(async move {
                    let mut fetched = fetch_others(
                        &client,
                        &input.watched,
                        &input.known_others,
                        &input.others_synced_until,
                    );
                    fill_stacks(&mut fetched.pull_requests, &details);
                    let mut cache = cache.lock();
                    cache.replace_list(PullRequestListKey::OTHERS, &fetched.pull_requests)?;
                    cache.delete_expired(ttl)?;
                    anyhow::Ok(fetched)
                })
                .await;

            // Show the result and arm the next refresh; a closed view simply drops it.
            queue
                .update(cx, |queue, cx| match others {
                    Ok(others) => queue.show_others(others, cx),
                    Err(error) => queue.finish_refresh(Some(&error), None, cx),
                })
                .ok();
        }));
    }

    /// Shows the viewer's refreshed own lists, with the stacks just read
    /// filled into the Others list known so far; the refresh goes on.
    fn show_own_lists(&mut self, own: OwnLists, cx: &mut Context<Self>) {
        if let Some(error) = &own.details_error {
            eprintln!("branches and stacks: keeping the last known ones: {error}");
        }
        let mut others = self.fetched.others.clone();
        fill_stacks(&mut others, &own.details);
        self.replace_lists(
            QueueLists {
                authored: own.authored,
                review_requested: own.review_requested,
                reviewed: own.reviewed,
                others,
            },
            cx,
        );
    }

    /// Shows others' refreshed pull requests and ends the refresh.
    fn show_others(&mut self, others: FetchedOthers, cx: &mut Context<Self>) {
        self.others_tab_mut().set_error(others.error);
        self.others_tab_mut().set_synced_until(others.synced_until);
        let lists = QueueLists {
            authored: std::mem::take(&mut self.fetched.authored),
            review_requested: std::mem::take(&mut self.fetched.review_requested),
            reviewed: std::mem::take(&mut self.fetched.reviewed),
            others: others.pull_requests,
        };
        self.replace_lists(lists, cx);
        self.finish_refresh(None, others.retry_after, cx);
    }

    /// Ends a refresh: says why it failed (`failure`), holds the next one
    /// back while GitHub refuses requests (`retry_after`, or the failure's:
    /// until a used-up budget refills or as long as GitHub asked), then
    /// runs a refresh asked for meanwhile or arms the timed one.
    fn finish_refresh(
        &mut self,
        failure: Option<&anyhow::Error>,
        retry_after: Option<Duration>,
        cx: &mut Context<Self>,
    ) {
        // The outcome; a successful one checks the listed stacks with GitHub.
        if let Some(error) = failure {
            self.refresh_status = RefreshStatus::Failed(format!("{error:#}").into());
        } else {
            self.refresh_status = RefreshStatus::Idle;
            self.refresh_stacks(cx);
        }

        // Record when it finished, and how long GitHub refuses requests.
        let now = Instant::now();
        self.refresh_schedule.record_finished(now);
        let retry_after = retry_after.or_else(|| {
            failure
                .and_then(|error| error.downcast_ref::<GitHubError>())
                .and_then(GitHubError::retry_after)
        });
        if let Some(wait) = retry_after {
            self.refresh_schedule.hold_until(now + wait);
        }

        // A refresh asked for meanwhile runs now, unless GitHub refuses requests: then the timer runs it.
        let runs_requested =
            std::mem::take(&mut self.is_refresh_requested) && !self.refresh_schedule.is_held(now);
        if runs_requested {
            self.refresh(cx);
        } else {
            self.schedule_next_refresh(cx);
        }
        cx.notify();
    }

    /// Starts the timer for the next refresh: an interval after the last
    /// one, or once GitHub allows requests again. A failed refresh is retried
    /// on the same timer, not sooner.
    fn schedule_next_refresh(&mut self, cx: &mut Context<Self>) {
        let delay = self.refresh_schedule.delay_until_next(Instant::now());

        self.auto_refresh_task = Some(cx.spawn(async move |queue, cx| {
            // Sleep until the deadline, then refresh; a closed view simply drops it.
            cx.background_executor().timer(delay).await;
            queue.update(cx, Self::refresh).ok();
        }));
    }

    /// Refreshes a stale queue when the window becomes active.
    pub(super) fn on_window_activation_changed(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Deactivation needs nothing; only coming back to the window counts.
        if !window.is_window_active() {
            return;
        }
        match self
            .refresh_schedule
            .decide_on_activation(&self.refresh_status, Instant::now())
        {
            ActivationRefresh::Refresh => self.refresh(cx),
            ActivationRefresh::NotDue | ActivationRefresh::AlreadyRefreshing => {}
        }
    }

    /// Replaces the lists after a refresh (or a change of the hidden branch
    /// rules), keeping the selection where the pull request is still shown
    /// (or was opened from a stack).
    pub(super) fn replace_lists(&mut self, fetched: QueueLists, cx: &mut Context<Self>) {
        self.hidden_branch_rules = AppSettings::hidden_branch_rules(cx).to_vec();
        let lists = without_hidden_branches(&fetched, &self.hidden_branch_rules);
        self.fetched = fetched;
        let refreshed = self
            .selected
            .as_ref()
            .and_then(|selected| lists.find_by_id(selected.id))
            .cloned();
        let selection = SelectionAfterRefresh::decide(
            self.selected.is_some(),
            self.selection_origin,
            refreshed.as_ref(),
        );
        self.review_tab = lists.review_tab();
        self.lists = lists;
        self.update_others_shown();
        self.request_author_avatars(cx);
        self.rebuild_rows();

        // The detail pane reloads an updated pull request and keeps its scroll.
        match selection {
            SelectionAfterRefresh::Cleared => {
                if self.selected.is_some() {
                    self.set_selected(None, SelectionOrigin::Listed, None, cx);
                    self.record_view_state(cx);
                }
            }
            SelectionAfterRefresh::Unchanged => {}
            SelectionAfterRefresh::Refreshed(pull_request) => {
                let pull_request = pull_request.clone();
                self.set_selected(Some(pull_request), SelectionOrigin::Listed, None, cx);
            }
        }

        // A selection saved before the relaunch gets its last chance here.
        self.restore_selection(RestorePhase::RefreshedQueue, cx);
        cx.notify();
    }
}
