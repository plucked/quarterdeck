use qd_github::PullRequest;

use super::restore_phase::RestorePhase;

/// What to do with a selection saved before the last relaunch.
#[derive(Debug, PartialEq, Eq)]
pub enum SelectionRestore<'found> {
    /// The pull request was found: select it.
    Select(&'found PullRequest),
    /// Not found yet; it may appear once the lists are refreshed.
    WaitForRefresh,
    /// Not in the refreshed lists either (merged, closed, review done): drop it.
    Abandon,
}

impl<'found> SelectionRestore<'found> {
    /// Decides from the saved pull request as found (in a list, or cached
    /// on its own) and which lists were searched.
    pub fn decide(found: Option<&'found PullRequest>, phase: RestorePhase) -> Self {
        match (found, phase) {
            (Some(pull_request), RestorePhase::CachedQueue | RestorePhase::RefreshedQueue) => {
                Self::Select(pull_request)
            }
            (None, RestorePhase::CachedQueue) => Self::WaitForRefresh,
            (None, RestorePhase::RefreshedQueue) => Self::Abandon,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qd_github::{RepoRef, User};

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
            labels: Vec::new(),
            stack: None,
        }
    }

    /// A saved pull request that was found is selected.
    #[test]
    fn found_pull_request_is_selected() {
        let found = pull_request(2);
        assert_eq!(
            SelectionRestore::decide(Some(&found), RestorePhase::CachedQueue),
            SelectionRestore::Select(&found)
        );
    }

    /// A pull request missing from the cached lists waits for the refresh.
    #[test]
    fn missing_from_cache_waits_for_refresh() {
        assert_eq!(
            SelectionRestore::decide(None, RestorePhase::CachedQueue),
            SelectionRestore::WaitForRefresh
        );
    }

    /// A pull request missing from the refreshed lists is abandoned.
    #[test]
    fn missing_after_refresh_is_abandoned() {
        assert_eq!(
            SelectionRestore::decide(None, RestorePhase::RefreshedQueue),
            SelectionRestore::Abandon
        );
    }
}
