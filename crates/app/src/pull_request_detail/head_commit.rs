use anyhow::Result;
use qd_github::PullRequest;

use super::review_target::ReviewTarget;
use crate::shared_cache::SharedCache;

/// The head commit the shown files are of, named without reading it yet:
/// reading a pull request's takes the cache, so it is resolved only where
/// needed, on the background executor.
pub enum HeadCommit {
    /// The head commit the pull request's cached files were fetched for.
    OfPullRequest(u64),
    /// A known commit: a combined range's head.
    Known(String),
}

impl HeadCommit {
    /// The head of the files shown for `pull_request` at `target`: a
    /// combined range's own head, else the pull request's cached files' one.
    pub fn of(target: &ReviewTarget, pull_request: &PullRequest) -> Self {
        match target {
            ReviewTarget::Combined(range) => Self::Known(range.head_sha.clone()),
            ReviewTarget::Layer | ReviewTarget::AwaitingStack => {
                Self::OfPullRequest(pull_request.id)
            }
        }
    }

    /// The commit's SHA; `None` while the pull request's files were never
    /// cached with their commits.
    ///
    /// Blocking: reads the cache for a pull request's head.
    pub fn resolve(&self, cache: &SharedCache) -> Result<Option<String>> {
        match self {
            Self::Known(commit) => Ok(Some(commit.clone())),
            Self::OfPullRequest(pull_request_id) => {
                let commits = cache.lock().file_set_commits(*pull_request_id)?;
                Ok(commits.map(|commits| commits.head_sha))
            }
        }
    }
}
