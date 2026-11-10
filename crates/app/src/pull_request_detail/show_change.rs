use qd_github::{PullRequest, PullRequestStack};

/// What showing a pull request changes, decided from what was shown before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowChange {
    /// The version shown already: only its data is taken, and its stack is
    /// loaded when `loads_stack` (its membership arrived only now).
    SameVersion { loads_stack: bool },
    /// A new version of the shown pull request: its files reload where the
    /// reader is; `is_other_stack` when the loaded stack's layers no longer apply.
    NewVersion { is_other_stack: bool },
    /// Another pull request: it starts at the top (or at a restored
    /// position); `is_other_stack` as for a new version.
    OtherPullRequest { is_other_stack: bool },
}

impl ShowChange {
    /// What showing `next` changes after `shown`, whose loaded stack is `stack`.
    pub fn decide(
        shown: Option<&PullRequest>,
        stack: Option<&PullRequestStack>,
        next: &PullRequest,
    ) -> Self {
        let is_other_stack = !is_same_stack(stack, next);
        let Some(shown) = shown.filter(|shown| shown.id == next.id) else {
            return Self::OtherPullRequest { is_other_stack };
        };
        if shown.updated_at == next.updated_at {
            Self::SameVersion {
                loads_stack: is_other_stack && next.stack.is_some(),
            }
        } else {
            Self::NewVersion { is_other_stack }
        }
    }
}

/// Whether `pull_request` belongs to `stack` (the stack shown so far); no
/// stack and no membership count as the same.
fn is_same_stack(stack: Option<&PullRequestStack>, pull_request: &PullRequest) -> bool {
    match (stack, &pull_request.stack) {
        (Some(stack), Some(membership)) => {
            stack.repo == pull_request.repo && stack.number == membership.number
        }
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

#[cfg(test)]
mod tests {
    use qd_github::{RepoRef, StackMembership, User};

    use super::*;

    fn repo() -> RepoRef {
        RepoRef {
            owner: "plucked".into(),
            name: "quarterdeck".into(),
        }
    }

    fn pull_request(id: u64, updated_at: &str, stack: Option<u64>) -> PullRequest {
        PullRequest {
            id,
            number: id,
            repo: repo(),
            title: "Title".into(),
            body: None,
            author: User {
                login: "someone".into(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft: false,
            created_at: "2026-11-01T00:00:00Z".into(),
            updated_at: updated_at.into(),
            labels: Vec::new(),
            stack: stack.map(|number| StackMembership {
                number,
                position: 1,
                size: 2,
                base_ref: "main".into(),
            }),
            branches: None,
            viewer_review: None,
        }
    }

    fn stack(number: u64) -> PullRequestStack {
        PullRequestStack {
            repo: repo(),
            number,
            base_ref: "main".into(),
            is_open: true,
            layers: Vec::new(),
        }
    }

    /// The version shown again changes nothing, and loads no stack.
    #[test]
    fn the_same_version_changes_nothing() {
        let shown = pull_request(1, "a", None);
        assert_eq!(
            ShowChange::decide(Some(&shown), None, &pull_request(1, "a", None)),
            ShowChange::SameVersion { loads_stack: false }
        );
        let stacked = pull_request(1, "a", Some(9));
        assert_eq!(
            ShowChange::decide(Some(&stacked), Some(&stack(9)), &stacked),
            ShowChange::SameVersion { loads_stack: false }
        );
    }

    /// A stack membership that arrives with the same version loads the stack.
    #[test]
    fn a_membership_arriving_later_loads_the_stack() {
        let shown = pull_request(1, "a", None);
        assert_eq!(
            ShowChange::decide(Some(&shown), None, &pull_request(1, "a", Some(9))),
            ShowChange::SameVersion { loads_stack: true }
        );
    }

    /// A new version of the shown pull request reloads, keeping the stack it is in.
    #[test]
    fn a_new_version_reloads() {
        let shown = pull_request(1, "a", Some(9));
        assert_eq!(
            ShowChange::decide(
                Some(&shown),
                Some(&stack(9)),
                &pull_request(1, "b", Some(9))
            ),
            ShowChange::NewVersion {
                is_other_stack: false
            }
        );
        assert_eq!(
            ShowChange::decide(Some(&shown), Some(&stack(9)), &pull_request(1, "b", None)),
            ShowChange::NewVersion {
                is_other_stack: true
            }
        );
    }

    /// Another pull request, or the first one, starts over; another stack's
    /// layers do not apply.
    #[test]
    fn another_pull_request_starts_over() {
        let shown = pull_request(1, "a", Some(9));
        assert_eq!(
            ShowChange::decide(
                Some(&shown),
                Some(&stack(9)),
                &pull_request(2, "a", Some(9))
            ),
            ShowChange::OtherPullRequest {
                is_other_stack: false
            }
        );
        assert_eq!(
            ShowChange::decide(
                Some(&shown),
                Some(&stack(9)),
                &pull_request(2, "a", Some(4))
            ),
            ShowChange::OtherPullRequest {
                is_other_stack: true
            }
        );
        assert_eq!(
            ShowChange::decide(None, None, &pull_request(2, "a", None)),
            ShowChange::OtherPullRequest {
                is_other_stack: false
            }
        );
    }
}
