use anyhow::{Context as _, Result};
use qd_diff::SourceLines;
use qd_github::{FileTextLocation, GitHubClient, PullRequestFile, RepoRef};

use crate::shared_cache::SharedCache;

/// Hex digits of a full git SHA.
const FULL_SHA_LENGTH: usize = 40;

/// The commit a file's text is read from when its blob SHA is not known in
/// full (files from a whole diff, or cached before SHAs were kept).
pub enum HeadCommit {
    /// The head commit the pull request's cached files were fetched for.
    OfPullRequest(u64),
    /// A known commit: a combined range's head.
    Known(String),
}

/// The text of `file` after the change, asking GitHub at most once per
/// blob: from the cache by the file's blob SHA (content addressed, so never
/// stale), else fetched (one request: by full SHA, or at the file's path in
/// `head`) and cached under the SHA GitHub reports.
///
/// Blocking: runs on the background executor; no lock is held during the request.
pub fn fetch_file_text(
    client: Result<GitHubClient, String>,
    cache: &SharedCache,
    repo: &RepoRef,
    file: &PullRequestFile,
    head: &HeadCommit,
) -> Result<SourceLines> {
    // A blob read before needs no request; an abbreviated SHA finds it too.
    if let Some(blob_sha) = &file.sha {
        let cached = cache.lock().file_text(blob_sha)?;
        if let Some(cached) = cached {
            return Ok(SourceLines::parse(&cached.text));
        }
    }

    // Ask by blob when its SHA is whole, else by path in the head commit.
    let client = client.map_err(anyhow::Error::msg)?;
    let location = match &file.sha {
        Some(blob_sha) if blob_sha.len() == FULL_SHA_LENGTH => {
            FileTextLocation::Blob(blob_sha.clone())
        }
        Some(_) | None => FileTextLocation::Path {
            commit: head_commit(cache, head)?,
            path: file.filename.clone(),
        },
    };
    let fetched = client.file_text(repo, &location)?;
    cache.lock().put_file_text(&fetched)?;
    Ok(SourceLines::parse(&fetched.text))
}

/// The commit `head` names.
fn head_commit(cache: &SharedCache, head: &HeadCommit) -> Result<String> {
    match head {
        HeadCommit::Known(commit) => Ok(commit.clone()),
        HeadCommit::OfPullRequest(pull_request_id) => {
            let commits = cache.lock().file_set_commits(*pull_request_id)?;
            commits
                .map(|commits| commits.head_sha)
                .context("the pull request's head commit is not known")
        }
    }
}
