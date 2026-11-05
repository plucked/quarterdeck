use qc_github::PullRequest;

use super::restore_phase::RestorePhase;

/// What to do with a selection saved before the last relaunch.
#[derive(Debug, PartialEq, Eq)]
pub enum SelectionRestore {
    /// The pull request is in the queue at this index: select it.
    Select(usize),
    /// Not in the cached queue; it may appear once the queue is refreshed.
    WaitForRefresh,
    /// Not in the refreshed queue either (merged, closed, review done): drop it.
    Abandon,
}

impl SelectionRestore {
    /// Decides how to restore the saved `pull_request_id` against `queue`.
    pub fn decide(pull_request_id: u64, queue: &[PullRequest], phase: RestorePhase) -> Self {
        let position = queue
            .iter()
            .position(|pull_request| pull_request.id == pull_request_id);
        match (position, phase) {
            (Some(index), RestorePhase::CachedQueue | RestorePhase::RefreshedQueue) => {
                Self::Select(index)
            }
            (None, RestorePhase::CachedQueue) => Self::WaitForRefresh,
            (None, RestorePhase::RefreshedQueue) => Self::Abandon,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qc_github::{RepoRef, User};

    fn pull_request(id: u64) -> PullRequest {
        PullRequest {
            id,
            number: id,
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            title: String::new(),
            body: None,
            author: User {
                login: String::new(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    /// A saved pull request still in the queue is selected at its current index.
    #[test]
    fn present_pull_request_is_selected() {
        let queue = [pull_request(1), pull_request(2)];
        assert_eq!(
            SelectionRestore::decide(2, &queue, RestorePhase::CachedQueue),
            SelectionRestore::Select(1)
        );
    }

    /// A pull request missing from the cached queue waits for the refresh.
    #[test]
    fn missing_from_cache_waits_for_refresh() {
        let queue = [pull_request(1)];
        assert_eq!(
            SelectionRestore::decide(9, &queue, RestorePhase::CachedQueue),
            SelectionRestore::WaitForRefresh
        );
    }

    /// A pull request missing from the refreshed queue is abandoned.
    #[test]
    fn missing_after_refresh_is_abandoned() {
        let queue = [pull_request(1)];
        assert_eq!(
            SelectionRestore::decide(9, &queue, RestorePhase::RefreshedQueue),
            SelectionRestore::Abandon
        );
    }
}
