use qc_github::PullRequest;

use super::selection_origin::SelectionOrigin;

/// What happens to the selection when the lists are replaced by a refresh.
#[derive(Debug, PartialEq, Eq)]
pub enum SelectionAfterRefresh<'lists> {
    /// Nothing was selected, or the listed pull request left every list.
    Cleared,
    /// The selection stays as it is (a stack layer in no list, or a pull
    /// request just merged or closed).
    Unchanged,
    /// The same pull request, as the refreshed list has it (its stack
    /// membership may be new); the detail reloads it if `updated_at` moved.
    Refreshed(&'lists PullRequest),
}

impl<'lists> SelectionAfterRefresh<'lists> {
    /// Decides the selection given whether something was selected before
    /// the refresh, where it came from, and the same id in the refreshed lists.
    pub fn decide(
        was_selected: bool,
        origin: SelectionOrigin,
        refreshed: Option<&'lists PullRequest>,
    ) -> Self {
        match (was_selected, refreshed, origin) {
            (false, _, _) | (true, None, SelectionOrigin::Listed) => Self::Cleared,
            (true, Some(refreshed), _) => Self::Refreshed(refreshed),
            (true, None, SelectionOrigin::StackLayer | SelectionOrigin::Finished) => {
                Self::Unchanged
            }
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
            stack: None,
        }
    }

    /// Without a previous selection nothing is selected afterwards.
    #[test]
    fn no_previous_selection_clears() {
        assert_eq!(
            SelectionAfterRefresh::decide(false, SelectionOrigin::Listed, None),
            SelectionAfterRefresh::Cleared
        );
    }

    /// A listed pull request that left every list clears the selection.
    #[test]
    fn removed_listed_pull_request_clears() {
        assert_eq!(
            SelectionAfterRefresh::decide(true, SelectionOrigin::Listed, None),
            SelectionAfterRefresh::Cleared
        );
    }

    /// A stack layer opened from the map stays selected though no list has it.
    #[test]
    fn unlisted_stack_layer_stays() {
        assert_eq!(
            SelectionAfterRefresh::decide(true, SelectionOrigin::StackLayer, None),
            SelectionAfterRefresh::Unchanged
        );
    }

    /// A pull request just merged or closed stays selected after leaving the lists.
    #[test]
    fn finished_pull_request_stays() {
        assert_eq!(
            SelectionAfterRefresh::decide(true, SelectionOrigin::Finished, None),
            SelectionAfterRefresh::Unchanged
        );
    }

    /// A pull request still listed is taken as the refresh has it.
    #[test]
    fn listed_pull_request_is_refreshed() {
        let refreshed = pull_request(1, "t2");
        assert_eq!(
            SelectionAfterRefresh::decide(true, SelectionOrigin::Listed, Some(&refreshed)),
            SelectionAfterRefresh::Refreshed(&refreshed)
        );
    }
}
