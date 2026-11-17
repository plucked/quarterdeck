use std::collections::HashMap;

use qd_github::PullRequest;

use super::collapsed_groups::CollapsedGroups;
use super::queue_row::QueueRow;
use super::restore_phase::RestorePhase;
use super::stack_key::StackKey;

/// The first review to open from Mine when launch has no saved selection to show.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum StartupSelection {
    /// The saved selection or first fetched queue may still resolve the launch.
    Wait,
    /// The first fetched queue resolved without needing a fallback.
    Complete,
    /// Open the stack's combined review, as a click on its header would.
    Stack(StackKey),
    /// Open a standalone pull request at this index in Mine.
    PullRequest(usize),
}

impl StartupSelection {
    /// Keeps a saved/current selection ahead of the default until the first refresh settles it.
    pub fn decide(
        selected: Option<&PullRequest>,
        pending_restore_id: Option<u64>,
        phase: RestorePhase,
        authored: &[PullRequest],
    ) -> Self {
        if pending_restore_id.is_some() {
            return Self::Wait;
        }
        match (selected, phase) {
            (Some(_), RestorePhase::Cached | RestorePhase::Refreshing) => Self::Wait,
            (Some(_), RestorePhase::Refreshed | RestorePhase::Unavailable) => Self::Complete,
            (None, RestorePhase::Cached | RestorePhase::Refreshing) => {
                Self::from_authored(authored).unwrap_or(Self::Wait)
            }
            (None, RestorePhase::Refreshed | RestorePhase::Unavailable) => {
                Self::from_authored(authored).unwrap_or(Self::Complete)
            }
        }
    }

    /// Chooses the first stack in sidebar order, else the first standalone pull request.
    fn from_authored(authored: &[PullRequest]) -> Option<Self> {
        // Repository grouping determines visual order; folded groups still contain candidates.
        let rows = QueueRow::build_all(authored, &CollapsedGroups::default(), &HashMap::new());
        rows.iter()
            .find_map(|row| match row {
                QueueRow::StackHeader(group) => Some(Self::Stack(group.key.clone())),
                _ => None,
            })
            .or_else(|| {
                rows.iter().find_map(|row| match row {
                    QueueRow::PullRequest(index) => Some(Self::PullRequest(*index)),
                    _ => None,
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use qd_github::{RepoRef, StackMembership, User};

    use super::*;

    /// A queue fixture whose input order is newest first, as the cache and API supply it.
    fn pull_request(repo: &str, number: u64, stack: Option<(u64, u64)>) -> PullRequest {
        PullRequest {
            id: number,
            number,
            repo: RepoRef {
                owner: "owner".into(),
                name: repo.into(),
            },
            title: String::new(),
            body: None,
            author: User {
                login: "author".into(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft: false,
            created_at: String::new(),
            updated_at: String::new(),
            branches: None,
            viewer_review: None,
            labels: Vec::new(),
            stack: stack.map(|(number, position)| StackMembership {
                number,
                position,
                size: 2,
                base_ref: "main".into(),
            }),
        }
    }

    /// A stack takes precedence over a newer standalone PR, in the sidebar's repository order.
    #[test]
    fn prefers_first_displayed_stack() {
        let authored = [
            pull_request("first", 1, None),
            pull_request("second", 2, Some((20, 1))),
            pull_request("first", 3, Some((10, 1))),
            pull_request("first", 4, Some((10, 2))),
        ];
        assert_eq!(
            StartupSelection::from_authored(&authored),
            Some(StartupSelection::Stack(StackKey::of(&authored[2]).unwrap()))
        );
    }

    /// Without a stack, the first standalone Mine PR is selected; an empty Mine stays empty.
    #[test]
    fn falls_back_to_first_standalone_pull_request() {
        let authored = [
            pull_request("first", 1, None),
            pull_request("second", 2, None),
        ];
        assert_eq!(
            StartupSelection::from_authored(&authored),
            Some(StartupSelection::PullRequest(0))
        );
        assert_eq!(StartupSelection::from_authored(&[]), None);
    }

    /// Cold-cache launch waits for Mine, then selects once the first refreshed list arrives.
    #[test]
    fn cold_cache_waits_for_mine() {
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Cached, &[]),
            StartupSelection::Wait
        );
        let authored = [pull_request("repo", 1, None)];
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Refreshed, &authored),
            StartupSelection::PullRequest(0)
        );
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Refreshed, &[]),
            StartupSelection::Complete
        );
    }

    /// A saved ID awaiting refresh takes precedence over an available Mine candidate.
    #[test]
    fn pending_restore_keeps_priority() {
        let authored = [pull_request("repo", 1, None)];
        assert_eq!(
            StartupSelection::decide(None, Some(2), RestorePhase::Cached, &authored),
            StartupSelection::Wait
        );
        // Once restoration abandons a missing ID, the refreshed Mine becomes the fallback.
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Refreshed, &authored),
            StartupSelection::PullRequest(0)
        );
    }

    /// A cached restored PR gets a fallback if the first refresh removes it; a valid one stays.
    #[test]
    fn cached_selection_waits_for_authoritative_refresh() {
        let selected = pull_request("repo", 2, None);
        let authored = [pull_request("repo", 1, None)];
        assert_eq!(
            StartupSelection::decide(Some(&selected), None, RestorePhase::Cached, &authored),
            StartupSelection::Wait
        );
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Refreshed, &authored),
            StartupSelection::PullRequest(0)
        );
        assert_eq!(
            StartupSelection::decide(Some(&selected), None, RestorePhase::Refreshed, &authored),
            StartupSelection::Complete
        );
    }

    /// When refresh is unavailable, cached Mine still opens and an empty queue stops waiting.
    #[test]
    fn unavailable_refresh_uses_cached_mine() {
        let authored = [pull_request("repo", 1, None)];
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Unavailable, &authored),
            StartupSelection::PullRequest(0)
        );
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Unavailable, &[]),
            StartupSelection::Complete
        );
    }

    /// A cached automatic pick can disappear before startup finishes and then choose fresh Mine.
    #[test]
    fn stale_automatic_pick_remains_eligible_during_startup() {
        let cached = [pull_request("repo", 1, None)];
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Cached, &cached),
            StartupSelection::PullRequest(0)
        );
        assert!(RestorePhase::Cached.is_provisional());
        let fresh = [pull_request("repo", 2, None)];
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Refreshing, &fresh),
            StartupSelection::PullRequest(0)
        );
        assert!(RestorePhase::Refreshing.is_provisional());
        assert_eq!(
            StartupSelection::decide(Some(&fresh[0]), None, RestorePhase::Refreshed, &fresh),
            StartupSelection::Complete
        );
        assert!(!RestorePhase::Refreshed.is_provisional());
    }

    /// Own-list arrival cannot settle a cached Others selection before Others has refreshed.
    #[test]
    fn others_selection_waits_for_complete_refresh() {
        let selected = pull_request("others", 2, None);
        let authored = [pull_request("mine", 1, None)];
        assert_eq!(
            StartupSelection::decide(Some(&selected), None, RestorePhase::Refreshing, &authored),
            StartupSelection::Wait
        );
        // Once Others removes that PR, the fallback is still available.
        assert_eq!(
            StartupSelection::decide(None, None, RestorePhase::Refreshing, &authored),
            StartupSelection::PullRequest(0)
        );
        // A saved Others ID absent from the cache gets the same full-refresh opportunity.
        assert_eq!(
            super::super::selection_restore::SelectionRestore::decide(
                None,
                RestorePhase::Refreshing
            ),
            super::super::selection_restore::SelectionRestore::WaitForRefresh
        );
        assert_eq!(
            StartupSelection::decide(None, Some(selected.id), RestorePhase::Refreshing, &authored),
            StartupSelection::Wait
        );
    }
}
