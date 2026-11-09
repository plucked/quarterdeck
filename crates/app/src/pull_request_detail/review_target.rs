use qd_cache::{ReviewScope, StackReviewMode};
use qd_github::{PullRequest, PullRequestStack, RepoRef};

/// What the detail pane shows the files of: the shown pull request alone,
/// or several layers of its stack combined into one diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewTarget {
    /// The pull request's own files.
    Layer,
    /// A combined review was asked for, but the stack's layers are not
    /// known yet; the files wait for them.
    AwaitingStack,
    Combined(CombinedRange),
}

/// A range of layers reviewed as one diff: from the base of the first layer
/// to the head of the last (the shown pull request).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedRange {
    pub repo: RepoRef,
    pub base_sha: String,
    pub head_sha: String,
    /// The pull request numbers of the first and last layer.
    pub first_number: u64,
    pub last_number: u64,
    /// The 0-based positions of the first and last layer in the stack.
    pub first_index: usize,
    pub last_index: usize,
}

impl CombinedRange {
    /// How many layers the range covers.
    pub fn layer_count(&self) -> usize {
        self.last_index - self.first_index + 1
    }
}

impl ReviewTarget {
    /// The target for `pull_request` in `mode`, given its stack when loaded.
    /// The range ends at the shown layer and starts at `range_start` (a
    /// pull request number) when that layer is at or below it, else at the
    /// lowest layer not merged yet: merged layers are already in the trunk.
    /// A standalone pull request, or one its stack does not list, shows alone.
    pub fn decide(
        mode: StackReviewMode,
        pull_request: &PullRequest,
        stack: Option<&PullRequestStack>,
        range_start: Option<u64>,
    ) -> Self {
        if mode == StackReviewMode::Layer || pull_request.stack.is_none() {
            return Self::Layer;
        }
        let Some(stack) = stack else {
            return Self::AwaitingStack;
        };
        let Some(last_index) = stack
            .layers
            .iter()
            .position(|layer| layer.number == pull_request.number)
        else {
            return Self::Layer;
        };

        // Start where asked when it is not above the shown layer; else at the lowest unmerged layer.
        let asked_start = range_start.and_then(|number| {
            stack.layers[..=last_index]
                .iter()
                .position(|layer| layer.number == number)
        });
        let lowest_unmerged = stack.layers[..=last_index]
            .iter()
            .position(|layer| !layer.is_merged)
            .unwrap_or(last_index);
        let first_index = asked_start.unwrap_or(lowest_unmerged);
        let first = &stack.layers[first_index];
        let last = &stack.layers[last_index];
        Self::Combined(CombinedRange {
            repo: stack.repo.clone(),
            base_sha: first.base_sha.clone(),
            head_sha: last.head_sha.clone(),
            first_number: first.number,
            last_number: last.number,
            first_index,
            last_index,
        })
    }

    /// Whose viewed marks the files show: the pull request's, or the range's own.
    pub fn viewed_scope(&self, pull_request: &PullRequest) -> ReviewScope {
        match self {
            Self::Layer | Self::AwaitingStack => ReviewScope::PullRequest(pull_request.id),
            Self::Combined(range) => ReviewScope::CombinedLayers {
                repo: range.repo.clone(),
                first_number: range.first_number,
                last_number: range.last_number,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use qd_github::{StackLayer, StackMembership, User};

    use super::*;

    fn repo() -> RepoRef {
        RepoRef {
            owner: "a".into(),
            name: "b".into(),
        }
    }

    fn pull_request(number: u64, is_stacked: bool) -> PullRequest {
        PullRequest {
            id: number * 100,
            number,
            repo: repo(),
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
            branches: None,
            labels: Vec::new(),
            stack: is_stacked.then(|| StackMembership {
                number: 9,
                position: 1,
                size: 3,
                base_ref: "main".into(),
            }),
        }
    }

    fn layer(number: u64, is_merged: bool) -> StackLayer {
        StackLayer {
            pull_request_id: number,
            number,
            title: String::new(),
            is_open: !is_merged,
            is_draft: false,
            is_merged,
            html_url: String::new(),
            head_ref: String::new(),
            head_sha: format!("head-{number}"),
            base_ref: String::new(),
            base_sha: format!("base-{number}"),
            author_login: String::new(),
        }
    }

    fn stack(layers: Vec<StackLayer>) -> PullRequestStack {
        PullRequestStack {
            repo: repo(),
            number: 9,
            base_ref: "main".into(),
            is_open: true,
            layers,
        }
    }

    fn combined(target: ReviewTarget) -> CombinedRange {
        let ReviewTarget::Combined(range) = target else {
            panic!("expected a combined range, got {target:?}");
        };
        range
    }

    /// Layer mode and standalone pull requests show the pull request alone.
    #[test]
    fn layer_mode_and_standalone_show_alone() {
        let stack = stack(vec![layer(1, false)]);
        let layer_mode = ReviewTarget::decide(
            StackReviewMode::Layer,
            &pull_request(1, true),
            Some(&stack),
            None,
        );
        assert_eq!(layer_mode, ReviewTarget::Layer);
        let standalone = ReviewTarget::decide(
            StackReviewMode::Combined,
            &pull_request(1, false),
            None,
            None,
        );
        assert_eq!(standalone, ReviewTarget::Layer);
    }

    /// A combined review waits for the stack's layers.
    #[test]
    fn combined_waits_for_stack() {
        let target = ReviewTarget::decide(
            StackReviewMode::Combined,
            &pull_request(2, true),
            None,
            None,
        );
        assert_eq!(target, ReviewTarget::AwaitingStack);
    }

    /// By default the range runs from the lowest unmerged layer to the shown one.
    #[test]
    fn default_range_skips_merged_layers() {
        let stack = stack(vec![
            layer(1, true),
            layer(2, false),
            layer(3, false),
            layer(4, false),
        ]);
        let range = combined(ReviewTarget::decide(
            StackReviewMode::Combined,
            &pull_request(3, true),
            Some(&stack),
            None,
        ));
        assert_eq!((range.first_number, range.last_number), (2, 3));
        assert_eq!(
            (range.base_sha.as_str(), range.head_sha.as_str()),
            ("base-2", "head-3")
        );
        assert_eq!(range.layer_count(), 2);
    }

    /// An asked start below the shown layer is used; one above it is ignored.
    #[test]
    fn asked_start_must_be_below_the_shown_layer() {
        let stack = stack(vec![layer(1, false), layer(2, false), layer(3, false)]);
        let shown = pull_request(2, true);
        let from_first = combined(ReviewTarget::decide(
            StackReviewMode::Combined,
            &shown,
            Some(&stack),
            Some(1),
        ));
        assert_eq!(from_first.first_number, 1);
        let from_above = combined(ReviewTarget::decide(
            StackReviewMode::Combined,
            &shown,
            Some(&stack),
            Some(3),
        ));
        assert_eq!(from_above.first_number, 1);
        let from_itself = combined(ReviewTarget::decide(
            StackReviewMode::Combined,
            &shown,
            Some(&stack),
            Some(2),
        ));
        assert_eq!(from_itself.layer_count(), 1);
    }
}
