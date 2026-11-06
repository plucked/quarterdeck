use std::time::{Duration, Instant};

use super::activation_refresh::ActivationRefresh;
use super::refresh_status::RefreshStatus;

/// How long after a refresh the next timed refresh runs.
const AUTO_REFRESH_INTERVAL: Duration = Duration::from_mins(5);
/// How old the queue must be for a window activation to refresh it, so
/// switching between apps does not spend the search API's rate limit.
const ACTIVATION_STALE_AFTER: Duration = Duration::from_mins(1);

/// When the review queue refreshes on its own: a timer after each refresh,
/// and on window activation once the queue is stale; neither before GitHub
/// allows it again after asking to slow down. Takes `now` as a parameter so
/// the decisions are testable without a clock.
#[derive(Default)]
pub struct RefreshSchedule {
    /// When the last refresh finished, successfully or not.
    last_finished_at: Option<Instant>,
    /// Until when GitHub asked not to be sent requests (its `retry-after`).
    held_until: Option<Instant>,
}

impl RefreshSchedule {
    /// Records that a refresh finished at `now`; the next timed refresh counts from here.
    pub fn record_finished(&mut self, now: Instant) {
        self.last_finished_at = Some(now);
    }

    /// Holds refreshes on their own back until `until` (GitHub's `retry-after`).
    pub fn hold_until(&mut self, until: Instant) {
        self.held_until = Some(until);
    }

    /// Whether refreshes on their own are held back at `now`.
    pub fn is_held(&self, now: Instant) -> bool {
        self.held_until.is_some_and(|until| now < until)
    }

    /// How long to wait from `now` until the next timed refresh is due: an
    /// interval after the last one, or the end of a hold when that is later.
    pub fn delay_until_next(&self, now: Instant) -> Duration {
        let elapsed = self.elapsed_since_finished(now);
        let interval_delay = AUTO_REFRESH_INTERVAL.saturating_sub(elapsed);
        let hold_delay = self
            .held_until
            .map_or(Duration::ZERO, |until| until.saturating_duration_since(now));
        interval_delay.max(hold_delay)
    }

    /// Decides whether a window activation at `now` refreshes the queue.
    pub fn decide_on_activation(&self, status: &RefreshStatus, now: Instant) -> ActivationRefresh {
        match status {
            RefreshStatus::Refreshing => ActivationRefresh::AlreadyRefreshing,
            RefreshStatus::Idle | RefreshStatus::Failed(_) if self.is_held(now) => {
                ActivationRefresh::NotDue
            }
            RefreshStatus::Idle | RefreshStatus::Failed(_) => {
                match self
                    .elapsed_since_finished(now)
                    .cmp(&ACTIVATION_STALE_AFTER)
                {
                    std::cmp::Ordering::Less => ActivationRefresh::NotDue,
                    std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => {
                        ActivationRefresh::Refresh
                    }
                }
            }
        }
    }

    /// Time since the last refresh finished; unbounded when none has.
    fn elapsed_since_finished(&self, now: Instant) -> Duration {
        self.last_finished_at.map_or(Duration::MAX, |finished_at| {
            now.saturating_duration_since(finished_at)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished_at(start: Instant) -> RefreshSchedule {
        let mut schedule = RefreshSchedule::default();
        schedule.record_finished(start);
        schedule
    }

    /// The timed refresh is due one interval after the last refresh finished.
    #[test]
    fn next_timed_refresh_counts_from_last_finish() {
        let start = Instant::now();
        let schedule = finished_at(start);
        let one_minute_later = start + Duration::from_mins(1);
        assert_eq!(
            schedule.delay_until_next(one_minute_later),
            Duration::from_mins(4)
        );
    }

    /// An overdue timed refresh runs at once instead of waiting a negative delay.
    #[test]
    fn overdue_timed_refresh_has_no_delay() {
        let start = Instant::now();
        let schedule = finished_at(start);
        let much_later = start + AUTO_REFRESH_INTERVAL * 2;
        assert_eq!(schedule.delay_until_next(much_later), Duration::ZERO);
    }

    /// Activating the window shortly after a refresh does not refresh again.
    #[test]
    fn activation_soon_after_refresh_is_not_due() {
        let start = Instant::now();
        let schedule = finished_at(start);
        assert_eq!(
            schedule.decide_on_activation(&RefreshStatus::Idle, start + Duration::from_secs(5)),
            ActivationRefresh::NotDue
        );
    }

    /// Activating the window once the queue is stale refreshes it.
    #[test]
    fn activation_after_stale_period_refreshes() {
        let start = Instant::now();
        let schedule = finished_at(start);
        assert_eq!(
            schedule.decide_on_activation(&RefreshStatus::Idle, start + ACTIVATION_STALE_AFTER),
            ActivationRefresh::Refresh
        );
    }

    /// Activating the window while a refresh is in flight leaves it running.
    #[test]
    fn activation_during_refresh_keeps_it_running() {
        let schedule = RefreshSchedule::default();
        assert_eq!(
            schedule.decide_on_activation(&RefreshStatus::Refreshing, Instant::now()),
            ActivationRefresh::AlreadyRefreshing
        );
    }

    /// While GitHub asked to wait, neither the timer nor an activation refreshes.
    #[test]
    fn a_hold_delays_timed_and_activation_refreshes() {
        let start = Instant::now();
        let mut schedule = finished_at(start);
        schedule.hold_until(start + Duration::from_mins(10));
        assert_eq!(schedule.delay_until_next(start), Duration::from_mins(10));
        assert_eq!(
            schedule.decide_on_activation(&RefreshStatus::Idle, start + Duration::from_mins(2)),
            ActivationRefresh::NotDue
        );
    }

    /// Before any refresh finished, an activation refreshes.
    #[test]
    fn activation_without_previous_refresh_refreshes() {
        let schedule = RefreshSchedule::default();
        assert_eq!(
            schedule.decide_on_activation(&RefreshStatus::Idle, Instant::now()),
            ActivationRefresh::Refresh
        );
    }
}
