//! Sample GitHub data the crate's tests store and read back.

use qd_github::{PullRequest, PullRequestFile, RepoRef, User};

/// Pull request `id` (its number the same) in `a/b` by `octo`, titled
/// `PR {id}`, last updated at `updated_at`; without a body, branches, a
/// review, labels or a stack.
pub(crate) fn sample_pull_request(id: u64, updated_at: &str) -> PullRequest {
    PullRequest {
        id,
        number: id,
        repo: RepoRef {
            owner: "a".into(),
            name: "b".into(),
        },
        title: format!("PR {id}"),
        body: None,
        author: User {
            login: "octo".into(),
            avatar_url: String::new(),
        },
        html_url: String::new(),
        draft: false,
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: updated_at.into(),
        branches: None,
        viewer_review: None,
        labels: Vec::new(),
        stack: None,
    }
}

/// A modified file `filename`, one line added and one removed, with a
/// one-line patch and no blob SHA.
pub(crate) fn sample_file(filename: &str) -> PullRequestFile {
    PullRequestFile {
        filename: filename.into(),
        previous_filename: None,
        status: "modified".into(),
        additions: 1,
        deletions: 1,
        patch: Some("@@ -1 +1 @@".into()),
        sha: None,
    }
}
