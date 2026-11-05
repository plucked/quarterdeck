use anyhow::Result;
use qc_github::{GitHubClient, PullRequest, PullRequestFile};

use super::file_set_update::FileSetUpdate;
use crate::shared_cache::SharedCache;

/// The changed files of `pull_request`, asking GitHub as little as possible:
///
/// 1. Files cached for the pull request's current `updated_at`: no request.
/// 2. Otherwise one request for its head and base commits, conditional on the
///    tag of the last such answer (free when unchanged). Unchanged commits
///    keep the cached files: a comment or review moved `updated_at`.
/// 3. Only new commits (or nothing cached) fetch the files, every page.
///
/// Blocking: runs on the background executor. Each cache access locks on its
/// own, so no lock is held during a request.
pub fn fetch_changed_files(
    client: Result<GitHubClient, String>,
    cache: &SharedCache,
    pull_request: &PullRequest,
) -> Result<Vec<PullRequestFile>> {
    // Step 1: a set confirmed for this version needs no request.
    let cached = cache.lock().file_set(pull_request.id)?;
    if let Some(set) = &cached
        && set.is_current_for(pull_request)
    {
        return Ok(cached.map(|set| set.files).unwrap_or_default());
    }

    // Step 2: ask for the commits, free when nothing changed since the tag.
    let client = client.map_err(anyhow::Error::msg)?;
    let etag = cached.as_ref().and_then(|set| set.revision.etag.clone());
    let answer =
        client.pull_request_revision(&pull_request.repo, pull_request.number, etag.as_ref())?;
    let mut update = FileSetUpdate::decide(cached, answer);
    if update == FileSetUpdate::AskWithoutTag {
        let answer = client.pull_request_revision(&pull_request.repo, pull_request.number, None)?;
        update = FileSetUpdate::decide(None, answer);
    }

    // Step 3: keep the cached files, or fetch them for the new commits.
    match update {
        FileSetUpdate::Confirm { files, revision } => {
            cache.lock().confirm_files(pull_request, &revision)?;
            Ok(files)
        }
        FileSetUpdate::Refetch { revision } => {
            let files = client.pull_request_files(&pull_request.repo, pull_request.number)?;
            cache.lock().put_files(pull_request, &files, &revision)?;
            Ok(files)
        }
        // Asked again without a tag, GitHub cannot answer "not modified".
        FileSetUpdate::AskWithoutTag => Ok(Vec::new()),
    }
}
