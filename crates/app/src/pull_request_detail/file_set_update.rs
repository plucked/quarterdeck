use qc_cache::{CachedFileSet, FileSetRevision};
use qc_github::{ConditionalResponse, PullRequestFile, RevisionCheck};

/// What to do with a pull request's cached files after asking GitHub for
/// its current commits.
#[derive(Debug, PartialEq, Eq)]
pub enum FileSetUpdate {
    /// The commits did not change: the cached files are current. Record the
    /// confirmation with this revision (its tag may be newer).
    Confirm {
        files: Vec<PullRequestFile>,
        revision: FileSetRevision,
    },
    /// The commits changed, or nothing is cached: fetch the files (GitHub
    /// counts `expected_files`) for this revision.
    Refetch {
        revision: FileSetRevision,
        expected_files: u64,
    },
    /// GitHub said "not modified" for a tag nothing cached belongs to; ask
    /// again without a tag.
    AskWithoutTag,
}

impl FileSetUpdate {
    /// Decides from the cached set (if any) and GitHub's answer about the commits.
    pub fn decide(
        cached: Option<CachedFileSet>,
        answer: ConditionalResponse<RevisionCheck>,
    ) -> Self {
        match (cached, answer) {
            // Unchanged since the tag: the cached revision still holds.
            (Some(cached), ConditionalResponse::NotModified) => Self::Confirm {
                files: cached.files,
                revision: cached.revision,
            },
            (None, ConditionalResponse::NotModified) => Self::AskWithoutTag,
            (Some(cached), ConditionalResponse::Modified { value, etag })
                if value.revision == cached.revision.commits =>
            {
                // Something else changed (a comment); the files did not.
                Self::Confirm {
                    files: cached.files,
                    revision: FileSetRevision {
                        commits: value.revision,
                        etag,
                    },
                }
            }
            (Some(_) | None, ConditionalResponse::Modified { value, etag }) => Self::Refetch {
                revision: FileSetRevision {
                    commits: value.revision,
                    etag,
                },
                expected_files: value.changed_files,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use qc_github::{ETag, PullRequestRevision};

    use super::*;

    fn commits(head: &str) -> PullRequestRevision {
        PullRequestRevision {
            head_sha: head.into(),
            base_sha: "base".into(),
        }
    }

    fn cached(head: &str) -> CachedFileSet {
        CachedFileSet {
            updated_at: "t1".into(),
            revision: FileSetRevision {
                commits: commits(head),
                etag: Some(ETag::new("old")),
            },
            files: Vec::new(),
        }
    }

    fn modified(head: &str) -> ConditionalResponse<RevisionCheck> {
        ConditionalResponse::Modified {
            value: RevisionCheck {
                revision: commits(head),
                changed_files: 4,
            },
            etag: Some(ETag::new("new")),
        }
    }

    /// A free "not modified" keeps the cached files and their revision.
    #[test]
    fn not_modified_confirms_cached_files() {
        assert_eq!(
            FileSetUpdate::decide(Some(cached("a")), ConditionalResponse::NotModified),
            FileSetUpdate::Confirm {
                files: Vec::new(),
                revision: cached("a").revision
            }
        );
    }

    /// A comment changes the pull request but not its commits: confirm with the new tag.
    #[test]
    fn same_commits_confirm_with_new_tag() {
        let FileSetUpdate::Confirm { revision, .. } =
            FileSetUpdate::decide(Some(cached("a")), modified("a"))
        else {
            panic!("expected a confirmation");
        };
        assert_eq!(revision.etag, Some(ETag::new("new")));
    }

    /// A push changes the head: refetch.
    #[test]
    fn new_commits_refetch() {
        assert!(matches!(
            FileSetUpdate::decide(Some(cached("a")), modified("b")),
            FileSetUpdate::Refetch { .. }
        ));
    }

    /// Nothing cached: refetch, or ask again if GitHub claims nothing changed.
    #[test]
    fn nothing_cached_refetches() {
        assert!(matches!(
            FileSetUpdate::decide(None, modified("a")),
            FileSetUpdate::Refetch { .. }
        ));
        assert_eq!(
            FileSetUpdate::decide(None, ConditionalResponse::NotModified),
            FileSetUpdate::AskWithoutTag
        );
    }
}
