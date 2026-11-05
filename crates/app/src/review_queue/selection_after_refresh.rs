use qc_github::PullRequest;

/// What happens to the selection when the queue is replaced by a refresh.
#[derive(Debug, PartialEq, Eq)]
pub enum SelectionAfterRefresh {
    /// Nothing was selected, or the selected pull request left the queue.
    Cleared,
    /// The same pull request, unchanged, now at this index.
    Kept(usize),
    /// The same pull request, now at this index, was updated since its files
    /// were loaded, so they are reloaded.
    KeptAndUpdated(usize),
}

impl SelectionAfterRefresh {
    /// Decides the selection for `refreshed`, given the pull request that was
    /// selected before the refresh.
    pub fn decide(previous: Option<&PullRequest>, refreshed: &[PullRequest]) -> Self {
        // Find the previously selected pull request by its stable id.
        let found = previous.and_then(|previous| {
            refreshed
                .iter()
                .position(|candidate| candidate.id == previous.id)
                .map(|index| (previous, index))
        });

        match found {
            None => Self::Cleared,
            Some((previous, index)) if refreshed[index].updated_at == previous.updated_at => {
                Self::Kept(index)
            }
            Some((_, index)) => Self::KeptAndUpdated(index),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qc_github::{RepoRef, User};

    fn pull_request(id: u64, updated_at: &str) -> PullRequest {
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
            updated_at: updated_at.into(),
        }
    }

    /// Without a previous selection nothing is selected afterwards.
    #[test]
    fn no_previous_selection_clears() {
        let refreshed = [pull_request(1, "t")];
        assert_eq!(
            SelectionAfterRefresh::decide(None, &refreshed),
            SelectionAfterRefresh::Cleared
        );
    }

    /// A selected pull request that left the queue clears the selection.
    #[test]
    fn removed_pull_request_clears() {
        let previous = pull_request(9, "t");
        let refreshed = [pull_request(1, "t")];
        assert_eq!(
            SelectionAfterRefresh::decide(Some(&previous), &refreshed),
            SelectionAfterRefresh::Cleared
        );
    }

    /// An unchanged pull request stays selected at its new index.
    #[test]
    fn unchanged_pull_request_is_kept_at_new_index() {
        let previous = pull_request(2, "t");
        let refreshed = [pull_request(1, "t"), pull_request(2, "t")];
        assert_eq!(
            SelectionAfterRefresh::decide(Some(&previous), &refreshed),
            SelectionAfterRefresh::Kept(1)
        );
    }

    /// An updated pull request stays selected and is marked for a file reload.
    #[test]
    fn updated_pull_request_is_kept_and_marked() {
        let previous = pull_request(1, "t1");
        let refreshed = [pull_request(1, "t2")];
        assert_eq!(
            SelectionAfterRefresh::decide(Some(&previous), &refreshed),
            SelectionAfterRefresh::KeptAndUpdated(0)
        );
    }
}
