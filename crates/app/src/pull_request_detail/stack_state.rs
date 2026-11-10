use gpui::{SharedString, Task};
use qd_github::PullRequestStack;

/// What the detail knows about the shown pull request's stack: its layers,
/// whether they are being read, why they could not be, the combined
/// range's chosen start and the load in flight. It belongs to one pull
/// request: showing nothing starts it over (`StackState::default()`, which
/// also drops and so cancels the load). The review mode (one layer or
/// combined) is the reader's preference and lives beside it, not in it.
#[derive(Default)]
pub struct StackState {
    /// The stack with its layers, once loaded (from the cache, then GitHub).
    pub(super) loaded: Option<PullRequestStack>,
    /// Whether the stack is being read or checked with GitHub.
    pub(super) is_loading: bool,
    /// Why the stack could not be checked with GitHub, if it could not.
    pub(super) error: Option<SharedString>,
    /// The first layer of a combined review (a pull request number), when
    /// chosen; else the range starts at the lowest unmerged layer.
    pub(super) combined_start: Option<u64>,
    /// The stack being loaded; replacing or dropping it cancels the load.
    pub(super) task: Option<Task<()>>,
}

impl StackState {
    /// Forgets the loaded layers and the combined range's start, which do
    /// not apply to another stack. A load in flight, whether one runs and
    /// its error stay: the load started for the shown pull request.
    pub(super) fn drop_layers(&mut self) {
        self.loaded = None;
        self.combined_start = None;
    }

    /// Cancels a load in flight and says none runs, for a pull request
    /// with no stack to load. The layers, error and range start stay.
    pub(super) fn stop_loading(&mut self) {
        self.task = None;
        self.is_loading = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state with every field but the layers and the task set.
    fn busy_state() -> StackState {
        StackState {
            is_loading: true,
            error: Some("rate limited".into()),
            combined_start: Some(7),
            ..StackState::default()
        }
    }

    /// Dropping the layers clears the range start but keeps the load's
    /// progress and error.
    #[test]
    fn dropping_layers_keeps_the_load() {
        let mut state = busy_state();
        state.drop_layers();
        assert!(state.loaded.is_none());
        assert_eq!(state.combined_start, None);
        assert!(state.is_loading);
        assert_eq!(state.error, Some(SharedString::from("rate limited")));
    }

    /// Stopping the load clears only whether one runs, not the error or
    /// the range start.
    #[test]
    fn stopping_the_load_keeps_the_rest() {
        let mut state = busy_state();
        state.stop_loading();
        assert!(!state.is_loading);
        assert!(state.task.is_none());
        assert_eq!(state.combined_start, Some(7));
        assert_eq!(state.error, Some(SharedString::from("rate limited")));
    }
}
