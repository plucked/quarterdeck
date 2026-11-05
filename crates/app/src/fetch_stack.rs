use anyhow::Result;
use qc_github::{ConditionalResponse, GitHubClient, PullRequestStack, RepoRef};

use crate::shared_cache::SharedCache;

/// The cached layers of stack `number` in `repo`, read quickly before
/// GitHub is asked; `None` when not cached.
///
/// Blocking: a local read, run on the background executor, or on the main
/// thread at startup (the queue's listed stacks).
pub fn cached_stack(
    cache: &SharedCache,
    repo: &RepoRef,
    number: u64,
) -> Result<Option<PullRequestStack>> {
    Ok(cache.lock().stack(repo, number)?.map(|cached| cached.stack))
}

/// The current layers of stack `number` in `repo`: one request, conditional
/// on the tag of the cached answer (free when nothing changed), caching a
/// new answer.
///
/// Blocking: runs on the background executor; no lock is held during the request.
pub fn refresh_stack(
    client: Result<GitHubClient, String>,
    cache: &SharedCache,
    repo: &RepoRef,
    number: u64,
) -> Result<PullRequestStack> {
    let cached = cache.lock().stack(repo, number)?;
    let client = client.map_err(anyhow::Error::msg)?;
    let etag = cached.as_ref().and_then(|cached| cached.etag.clone());
    match (
        client.pull_request_stack(repo, number, etag.as_ref())?,
        cached,
    ) {
        (ConditionalResponse::NotModified, Some(cached)) => {
            cache.lock().touch_stack(repo, number)?;
            Ok(cached.stack)
        }
        // "Not modified" for a tag nothing cached belongs to: ask plainly.
        (ConditionalResponse::NotModified, None) => {
            match client.pull_request_stack(repo, number, None)? {
                ConditionalResponse::Modified { value, etag } => {
                    cache.lock().put_stack(&value, etag.as_ref())?;
                    Ok(value)
                }
                ConditionalResponse::NotModified => {
                    anyhow::bail!("GitHub sent no stack {number} for an unconditional request")
                }
            }
        }
        (ConditionalResponse::Modified { value, etag }, _) => {
            cache.lock().put_stack(&value, etag.as_ref())?;
            Ok(value)
        }
    }
}
