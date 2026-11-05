use anyhow::Result;
use qd_github::{GitHubClient, MAX_LISTED_FILES, PullRequest, PullRequestFile};

use super::file_set_update::FileSetUpdate;
use super::load_progress::LoadProgress;
use crate::shared_cache::SharedCache;

/// Where a pull request's files come from.
#[derive(Debug, PartialEq, Eq)]
enum FileSource {
    /// GitHub's list of files, page by page (up to 3000 files).
    Pages,
    /// One download of the whole diff, for more files than GitHub lists.
    WholeDiff,
}

impl FileSource {
    /// The pages while GitHub lists every file, else the whole diff.
    fn for_file_count(changed_files: u64) -> Self {
        match changed_files.cmp(&MAX_LISTED_FILES) {
            std::cmp::Ordering::Less | std::cmp::Ordering::Equal => Self::Pages,
            std::cmp::Ordering::Greater => Self::WholeDiff,
        }
    }
}

/// The changed files of `pull_request`, asking GitHub as little as possible:
///
/// 1. Files cached for the pull request's current `updated_at`: no request.
/// 2. Otherwise one request for its head and base commits, conditional on the
///    tag of the last such answer (free when unchanged). Unchanged commits
///    keep the cached files: a comment or review moved `updated_at`.
/// 3. Only new commits (or nothing cached) fetch the files, every page.
///
/// GitHub lists at most 3000 files; a larger pull request's files come from
/// one download of its whole diff (base to head commit) instead.
///
/// Reports each step that talks to GitHub, and the files fetched so far.
///
/// Blocking: runs on the background executor. Each cache access locks on its
/// own, so no lock is held during a request.
pub fn fetch_changed_files(
    client: Result<GitHubClient, String>,
    cache: &SharedCache,
    pull_request: &PullRequest,
    on_progress: &mut dyn FnMut(LoadProgress),
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
    on_progress(LoadProgress::CheckingCommits);
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
        FileSetUpdate::Refetch {
            revision,
            expected_files,
        } => {
            let files = match FileSource::for_file_count(expected_files) {
                FileSource::Pages => {
                    on_progress(LoadProgress::FetchingFiles {
                        fetched: 0,
                        total: Some(expected_files),
                    });
                    client.pull_request_files(
                        &pull_request.repo,
                        pull_request.number,
                        &mut |fetched| {
                            on_progress(LoadProgress::FetchingFiles {
                                fetched,
                                total: Some(expected_files),
                            });
                        },
                    )?
                }
                FileSource::WholeDiff => client.compare_files(
                    &pull_request.repo,
                    &revision.commits.base_sha,
                    &revision.commits.head_sha,
                    &mut |download| on_progress(LoadProgress::DownloadingDiff(download)),
                )?,
            };
            cache.lock().put_files(pull_request, &files, &revision)?;
            Ok(files)
        }
        // Asked again without a tag, GitHub cannot answer "not modified".
        FileSetUpdate::AskWithoutTag => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Up to GitHub's 3000 the files come in pages; past it, as the whole diff.
    #[test]
    fn large_pull_requests_download_the_whole_diff() {
        assert_eq!(FileSource::for_file_count(12), FileSource::Pages);
        assert_eq!(FileSource::for_file_count(3000), FileSource::Pages);
        assert_eq!(FileSource::for_file_count(17_485), FileSource::WholeDiff);
    }
}
