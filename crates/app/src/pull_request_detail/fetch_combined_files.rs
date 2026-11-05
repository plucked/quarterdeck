use anyhow::Result;
use qd_github::{DownloadProgress, GitHubClient, PullRequestFile};

use super::review_target::CombinedRange;
use crate::shared_cache::SharedCache;

/// The changed files of a combined range of stack layers: from the cache
/// when this pair of commits was compared before (commits never change, so
/// no request), otherwise one request for the whole diff, then cached.
/// Reports the download as it arrives.
///
/// Blocking: runs on the background executor; no lock is held during the request.
pub fn fetch_combined_files(
    client: Result<GitHubClient, String>,
    cache: &SharedCache,
    range: &CombinedRange,
    on_progress: &mut dyn FnMut(DownloadProgress),
) -> Result<Vec<PullRequestFile>> {
    // The same commits compared before are still right.
    let cached = cache
        .lock()
        .compare_files(&range.repo, &range.base_sha, &range.head_sha)?;
    if let Some(files) = cached {
        return Ok(files);
    }

    // Ask GitHub once for the whole diff, and keep it.
    let client = client.map_err(anyhow::Error::msg)?;
    let files = client.compare_files(&range.repo, &range.base_sha, &range.head_sha, on_progress)?;
    cache
        .lock()
        .put_compare_files(&range.repo, &range.base_sha, &range.head_sha, &files)?;
    Ok(files)
}
