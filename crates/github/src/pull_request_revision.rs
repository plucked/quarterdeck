use serde::Deserialize;

use crate::RevisionCheck;

/// The commits a pull request compares: its head and its base. The changed
/// files only change when one of them does, unlike `updated_at`, which also
/// moves on every comment and review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestRevision {
    pub head_sha: String,
    pub base_sha: String,
}

/// The part of `GET /repos/{owner}/{repo}/pulls/{number}` that is read.
#[derive(Deserialize)]
pub(crate) struct PullRequestRefs {
    head: CommitRef,
    base: CommitRef,
    changed_files: u64,
    additions: u64,
    deletions: u64,
}

#[derive(Deserialize)]
struct CommitRef {
    sha: String,
}

impl PullRequestRefs {
    pub fn into_check(self) -> RevisionCheck {
        RevisionCheck {
            revision: PullRequestRevision {
                head_sha: self.head.sha,
                base_sha: self.base.sha,
            },
            changed_files: self.changed_files,
            additions: self.additions,
            deletions: self.deletions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The head and base commits and the file and line counts are read from a pull request response.
    #[test]
    fn reads_head_and_base_commits() {
        let json = r#"{"number": 1, "changed_files": 12, "additions": 250, "deletions": 40, "head": {"sha": "aaa", "ref": "x"}, "base": {"sha": "bbb", "ref": "main"}}"#;
        let refs: PullRequestRefs = serde_json::from_str(json).unwrap();
        assert_eq!(
            refs.into_check(),
            RevisionCheck {
                revision: PullRequestRevision {
                    head_sha: "aaa".into(),
                    base_sha: "bbb".into()
                },
                changed_files: 12,
                additions: 250,
                deletions: 40,
            }
        );
    }
}
