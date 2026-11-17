use anyhow::{Context as _, Result, bail};
use qd_github::{
    DownloadProgress, GitHubClient, GitHubError, MAX_LISTED_FILES, PullRequest, PullRequestFile,
};

use super::file_set_update::FileSetUpdate;
use super::load_progress::LoadProgress;
use crate::shared_cache::SharedCache;

/// Above this count, try one diff download before fetching individual API pages.
const DIFF_DOWNLOAD_FILE_THRESHOLD: u64 = 500;

/// Files and the two immutable commits that identify the displayed diff.
pub(super) struct FileSnapshot {
    pub files: Vec<PullRequestFile>,
    pub revision: qd_github::PullRequestRevision,
}

/// Where a pull request's files come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FileSource {
    /// GitHub's list of files, page by page (up to 3000 files).
    Pages,
    /// One download of the whole diff, preferred for larger pull requests.
    WholeDiff,
}

impl FileSource {
    /// API pages through 500 files; a diff download first above that threshold.
    fn for_file_count(changed_files: u64) -> Self {
        match changed_files.cmp(&DIFF_DOWNLOAD_FILE_THRESHOLD) {
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
/// 3. Only new commits (or nothing cached) fetch files. Above 500 files, try
///    the whole diff first, then API pages if GitHub has not prepared it.
///
/// Both sources must return the revision's full file count before caching.
/// An API fallback capped at 3000 is an error for a larger pull request;
/// GitHub throttling stops requests without attempting a fallback.
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
) -> Result<FileSnapshot> {
    let first = fetch_once(client.clone(), cache, pull_request, on_progress);
    if first
        .as_ref()
        .is_err_and(anyhow::Error::is::<RevisionChanged>)
    {
        fetch_once(client, cache, pull_request, on_progress)
    } else {
        first
    }
}

#[derive(Debug)]
struct RevisionChanged;
impl std::fmt::Display for RevisionChanged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            "The pull request changed during download. Refresh to review its current files.",
        )
    }
}
impl std::error::Error for RevisionChanged {}

fn fetch_once(
    client: Result<GitHubClient, String>,
    cache: &SharedCache,
    pull_request: &PullRequest,
    on_progress: &mut dyn FnMut(LoadProgress),
) -> Result<FileSnapshot> {
    // Step 1: a set confirmed for this version needs no request.
    let cached = cache.lock().file_set(pull_request.id)?;
    let cached = match cached {
        Some(set) if set.is_current_for(pull_request) => {
            return Ok(FileSnapshot {
                files: set.files,
                revision: set.revision.commits,
            });
        }
        other => other,
    };

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
            Ok(FileSnapshot {
                files,
                revision: revision.commits,
            })
        }
        FileSetUpdate::Refetch {
            revision,
            expected_files,
        } => {
            let files = fetch_uncached_files(expected_files, |source| match source {
                FileSource::Pages => {
                    on_progress(LoadProgress::FetchingFiles {
                        fetched: 0,
                        total: Some(expected_files),
                    });
                    let files = client.pull_request_files(
                        &pull_request.repo,
                        pull_request.number,
                        &mut |fetched| {
                            on_progress(LoadProgress::FetchingFiles {
                                fetched,
                                total: Some(expected_files),
                            });
                        },
                    )?;
                    let checked = client.pull_request_revision(
                        &pull_request.repo,
                        pull_request.number,
                        None,
                    )?;
                    verify_page_revision(&revision.commits, checked)?;
                    Ok(files)
                }
                FileSource::WholeDiff => {
                    on_progress(LoadProgress::DownloadingDiff(DownloadProgress {
                        received_bytes: 0,
                        total_bytes: None,
                    }));
                    client
                        .compare_files(
                            &pull_request.repo,
                            &revision.commits.base_sha,
                            &revision.commits.head_sha,
                            &mut |download| on_progress(LoadProgress::DownloadingDiff(download)),
                        )
                        .map_err(Into::into)
                }
            })?;
            cache.lock().put_files(pull_request, &files, &revision)?;
            Ok(FileSnapshot {
                files,
                revision: revision.commits,
            })
        }
        // Asked again without a tag, GitHub cannot answer "not modified".
        FileSetUpdate::AskWithoutTag => bail!("GitHub returned no revision to display"),
    }
}

fn verify_page_revision(
    expected: &qd_github::PullRequestRevision,
    answer: qd_github::ConditionalResponse<qd_github::RevisionCheck>,
) -> Result<()> {
    if matches!(answer, qd_github::ConditionalResponse::Modified {value, ..} if value.revision == *expected)
    {
        Ok(())
    } else {
        Err(RevisionChanged.into())
    }
}

/// Tries the preferred source, then API pages for an unavailable or incomplete diff.
fn fetch_uncached_files(
    expected_files: u64,
    mut fetch: impl FnMut(FileSource) -> Result<Vec<PullRequestFile>>,
) -> Result<Vec<PullRequestFile>> {
    // Validate at this boundary so no partial response can reach the file cache.
    let mut fetch_complete = |source| {
        let files = fetch(source)?;
        require_complete_files(source, expected_files, files)
    };
    match FileSource::for_file_count(expected_files) {
        FileSource::Pages => fetch_complete(FileSource::Pages),
        FileSource::WholeDiff => match fetch_complete(FileSource::WholeDiff) {
            Ok(files) => Ok(files),
            // A shared exhausted budget or slowdown also forbids the API fallback.
            Err(error)
                if error
                    .downcast_ref::<GitHubError>()
                    .is_some_and(GitHubError::is_throttled) =>
            {
                Err(error)
            }
            Err(diff_error) => fetch_complete(FileSource::Pages).with_context(|| {
                format!(
                    "The diff download failed ({diff_error:#}); the files API fallback also failed"
                )
            }),
        },
    }
}

/// Rejects a pending response, truncated API list, or revision that changed during loading.
fn require_complete_files(
    source: FileSource,
    expected_files: u64,
    files: Vec<PullRequestFile>,
) -> Result<Vec<PullRequestFile>> {
    let received_files = u64::try_from(files.len())?;
    if received_files != expected_files {
        match source {
            FileSource::Pages if expected_files > MAX_LISTED_FILES => bail!(
                "The files API returned {received_files} of {expected_files} files; GitHub lists at most {MAX_LISTED_FILES}. Refresh to retry the complete diff download."
            ),
            FileSource::Pages => bail!(
                "The files API returned {received_files} files, but the revision has {expected_files}. Refresh to load the current revision."
            ),
            FileSource::WholeDiff => bail!(
                "The diff download contained {received_files} files, but the revision has {expected_files}"
            ),
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use qd_github::GitHubError;

    use super::*;

    /// Creates one entry per changed path, without external requests or a user cache.
    fn files(count: usize) -> Vec<PullRequestFile> {
        (0..count)
            .map(|index| PullRequestFile {
                filename: format!("file-{index}.bin"),
                previous_filename: None,
                status: "modified".into(),
                additions: 0,
                deletions: 0,
                patch: None,
                sha: None,
            })
            .collect()
    }

    /// An ordinary API response whose diff is not available yet.
    fn pending_diff() -> anyhow::Error {
        GitHubError::Api {
            status: 503,
            body: "diff is being generated".into(),
        }
        .into()
    }

    /// At most 500 files use only the files API, including an empty revision.
    #[test]
    fn small_pull_requests_use_pages() {
        for count in [0, 12, 500] {
            let fetched = fetch_uncached_files(count, |source| {
                assert_eq!(source, FileSource::Pages);
                Ok(files(usize::try_from(count).unwrap()))
            })
            .unwrap();
            assert_eq!(u64::try_from(fetched.len()).unwrap(), count);
        }
    }

    /// More than 500 files prefer the diff, with no API request after complete success.
    #[test]
    fn large_pull_requests_download_the_whole_diff() {
        for count in [501, 3000, 3001] {
            let fetched = fetch_uncached_files(count, |source| {
                assert_eq!(source, FileSource::WholeDiff);
                Ok(files(usize::try_from(count).unwrap()))
            })
            .unwrap();
            assert_eq!(u64::try_from(fetched.len()).unwrap(), count);
        }
    }

    /// A diff still being generated falls back once to the normal files API.
    #[test]
    fn unavailable_diff_falls_back_to_pages() {
        let mut sources = Vec::new();
        let fetched = fetch_uncached_files(501, |source| {
            sources.push(source);
            match source {
                FileSource::WholeDiff => Err(pending_diff()),
                FileSource::Pages => Ok(files(501)),
            }
        })
        .unwrap();
        assert_eq!(fetched.len(), 501);
        assert_eq!(sources, [FileSource::WholeDiff, FileSource::Pages]);
    }

    /// Pending or incomplete successful diff responses also fall back instead of being cached.
    #[test]
    fn incomplete_diff_falls_back_to_pages() {
        for count in [0, 500, 502] {
            let mut sources = Vec::new();
            let fetched = fetch_uncached_files(501, |source| {
                sources.push(source);
                Ok(match source {
                    FileSource::WholeDiff => files(count),
                    FileSource::Pages => files(501),
                })
            })
            .unwrap();
            assert_eq!(fetched.len(), 501);
            assert_eq!(sources, [FileSource::WholeDiff, FileSource::Pages]);
        }
    }

    /// A fallback capped at 3000 cannot be published as a complete larger revision.
    #[test]
    fn truncated_api_fallback_is_an_error() {
        let mut sources = Vec::new();
        let error = fetch_uncached_files(3001, |source| {
            sources.push(source);
            match source {
                FileSource::WholeDiff => Err(pending_diff()),
                FileSource::Pages => Ok(files(3000)),
            }
        })
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("diff is being generated"), "{message}");
        assert!(message.contains("3000"), "{message}");
        assert!(message.contains("3001"), "{message}");
        assert_eq!(sources, [FileSource::WholeDiff, FileSource::Pages]);
    }

    /// If both requests fail, the result explains the diff failure and retains the API error type.
    #[test]
    fn failed_fallback_keeps_both_errors() {
        let error = fetch_uncached_files(501, |source| match source {
            FileSource::WholeDiff => Err(pending_diff()),
            FileSource::Pages => Err(GitHubError::Api {
                status: 502,
                body: "files API unavailable".into(),
            }
            .into()),
        })
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("diff is being generated"), "{message}");
        assert!(message.contains("files API unavailable"), "{message}");
        assert!(matches!(
            error.downcast_ref::<GitHubError>(),
            Some(GitHubError::Api { status: 502, .. })
        ));
    }

    /// Exhausted budgets and explicit slowdown responses stop without another request.
    #[test]
    fn throttled_diff_does_not_fall_back() {
        for throttle in [
            GitHubError::RateLimited {
                resource: "core",
                resets_at: 1000,
            },
            GitHubError::SlowDown {
                retry_after_seconds: Some(60),
            },
        ] {
            let mut throttle = Some(throttle);
            let error = fetch_uncached_files(501, |source| {
                assert_eq!(source, FileSource::WholeDiff);
                Err(throttle.take().expect("only one request").into())
            })
            .unwrap_err();
            assert!(error.downcast_ref::<GitHubError>().unwrap().is_throttled());
        }
    }
}

#[cfg(test)]
mod revision_tests {
    use super::*;
    use qd_github::{ConditionalResponse, PullRequestRevision, RevisionCheck};
    #[test]
    fn equal_counts_do_not_hide_a_push_or_rebase() {
        let rev = |head: &str, base: &str| PullRequestRevision {
            head_sha: head.into(),
            base_sha: base.into(),
        };
        for (head, base, valid) in [("h", "b", true), ("new", "b", false), ("h", "new", false)] {
            assert_eq!(
                verify_page_revision(
                    &rev("h", "b"),
                    ConditionalResponse::Modified {
                        value: RevisionCheck {
                            revision: rev(head, base),
                            changed_files: 2
                        },
                        etag: None
                    }
                )
                .is_ok(),
                valid
            );
        }
    }
}
