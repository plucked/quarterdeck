use crate::MergeMethod;

/// What merging or closing a pull request can do right now, as GitHub
/// reports it: its state, whether it merges cleanly, the methods the
/// repository allows, and what the viewer may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOptions {
    pub state: PullRequestState,
    pub readiness: MergeReadiness,
    /// The methods the repository allows, in GitHub's order.
    pub allowed_methods: Vec<MergeMethod>,
    pub permissions: MergePermissions,
    /// The repository deletes head branches after a merge by itself.
    pub deletes_branch_on_merge: bool,
    pub merge_queue: MergeQueueState,
    pub head_ref: String,
    /// The head commit the merge must still match.
    pub head_sha: String,
    pub base_ref: String,
    /// The head branch is in a fork, which the viewer cannot delete.
    pub is_cross_repository: bool,
}

/// Whether the base branch merges through a merge queue, and whether the
/// pull request is in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeQueueState {
    /// The base branch has no merge queue: merges are direct.
    NoQueue,
    /// A merge queue takes merges; the pull request is not in it.
    Available,
    /// The pull request waits in the merge queue.
    Queued,
}

/// Whether a pull request is open, closed or merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

/// Whether the pull request can be merged now (GitHub's
/// `mergeStateStatus`, with conflicts from `mergeable`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeReadiness {
    /// Mergeable and every requirement met.
    Clean,
    /// Mergeable; some checks failed or are still running.
    Unstable,
    /// Mergeable, with pre-receive hooks to pass.
    HasHooks,
    /// Requirements (reviews, checks, rules) are not met.
    Blocked,
    /// The head branch is behind the base and must be updated first.
    Behind,
    /// There are conflicts with the base branch.
    Conflicting,
    /// Still a draft.
    Draft,
    /// GitHub is still computing it.
    Unknown,
}

impl MergeReadiness {
    /// Decodes GitHub's `mergeStateStatus` and `mergeable`.
    pub(crate) fn from_github(merge_state: &str, mergeable: &str) -> Self {
        if mergeable == "CONFLICTING" {
            return Self::Conflicting;
        }
        match merge_state {
            "CLEAN" => Self::Clean,
            "UNSTABLE" => Self::Unstable,
            "HAS_HOOKS" => Self::HasHooks,
            "BLOCKED" => Self::Blocked,
            "BEHIND" => Self::Behind,
            "DIRTY" => Self::Conflicting,
            "DRAFT" => Self::Draft,
            _ => Self::Unknown,
        }
    }

    /// Whether a plain merge is allowed in this state.
    pub fn allows_merge(self) -> bool {
        match self {
            Self::Clean | Self::Unstable | Self::HasHooks => true,
            Self::Blocked | Self::Behind | Self::Conflicting | Self::Draft | Self::Unknown => false,
        }
    }

    /// What the state means, for the merge panel.
    pub fn description(self) -> &'static str {
        match self {
            Self::Clean => "Ready to merge.",
            Self::Unstable => "Mergeable, but some checks failed or are still running.",
            Self::HasHooks => "Mergeable; the repository's hooks still run.",
            Self::Blocked => "Blocked: required reviews, checks or rules are not met.",
            Self::Behind => "Behind its base branch: update the branch first.",
            Self::Conflicting => "Conflicts with its base branch.",
            Self::Draft => "A draft: mark it ready for review first.",
            Self::Unknown => "GitHub is still checking whether it can be merged.",
        }
    }
}

/// What the viewer may do with the pull request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MergePermissions {
    /// May merge past unmet requirements (an admin merge).
    pub can_merge_as_admin: bool,
    pub can_close: bool,
    /// May delete the head branch: GitHub says so, or (while the pull
    /// request is open, when GitHub never does) the viewer can push to the
    /// repository and the branch is not a fork's.
    pub can_delete_head_ref: bool,
}
