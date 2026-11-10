use qd_github::{PullRequestStack, RepoRef, StackMembership};

/// A stack layer to show: the pull request `number` of `repo`, with its
/// place in the stack (the issues API it may be fetched from lacks it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerRequest {
    pub repo: RepoRef,
    pub number: u64,
    pub membership: StackMembership,
}

impl LayerRequest {
    /// Layer `index` (bottom first, from 0) of `stack`; `None` when the
    /// stack has no such layer.
    pub fn of_layer(stack: &PullRequestStack, index: usize) -> Option<Self> {
        let layer = stack.layers.get(index)?;
        Some(Self {
            repo: stack.repo.clone(),
            number: layer.number,
            membership: StackMembership {
                number: stack.number,
                // Positions count from 1 at the bottom, as GitHub's do.
                position: u64::try_from(index + 1).unwrap_or(u64::MAX),
                size: u64::try_from(stack.layers.len()).unwrap_or(u64::MAX),
                base_ref: stack.base_ref.clone(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use qd_github::StackLayer;

    use super::*;

    fn layer(number: u64) -> StackLayer {
        StackLayer {
            pull_request_id: number,
            number,
            title: String::new(),
            is_open: true,
            is_draft: false,
            is_merged: false,
            html_url: String::new(),
            head_ref: String::new(),
            head_sha: String::new(),
            base_ref: String::new(),
            base_sha: String::new(),
            author_login: String::new(),
        }
    }

    /// A layer is asked for by number with its 1-based position; past the top there is none.
    #[test]
    fn requests_a_layer_by_its_place() {
        let stack = PullRequestStack {
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            number: 9,
            base_ref: "main".into(),
            is_open: true,
            layers: vec![layer(11), layer(12)],
        };
        let request = LayerRequest::of_layer(&stack, 1).unwrap();
        assert_eq!(request.number, 12);
        assert_eq!(request.membership.position, 2);
        assert_eq!(request.membership.size, 2);
        assert_eq!(request.membership.number, 9);
        assert_eq!(request.membership.base_ref, "main");
        assert!(LayerRequest::of_layer(&stack, 2).is_none());
    }
}
