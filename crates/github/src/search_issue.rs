use serde::Deserialize;

use crate::{PullRequest, RepoRef, User};

/// A pull request as `GET /search/issues` returns it: issue-shaped, with the
/// repository only as an API URL.
#[derive(Deserialize)]
pub(crate) struct SearchIssue {
    pub id: u64,
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub user: User,
    pub html_url: String,
    pub repository_url: String,
    #[serde(default)]
    pub draft: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl SearchIssue {
    /// Converts to a [`PullRequest`]; `None` when the repository URL is malformed.
    pub fn into_pull_request(self) -> Option<PullRequest> {
        Some(PullRequest {
            id: self.id,
            number: self.number,
            repo: RepoRef::from_api_url(&self.repository_url)?,
            title: self.title,
            body: self.body,
            author: self.user,
            html_url: self.html_url,
            draft: self.draft,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A search result maps to a pull request with its repository parsed from the URL.
    #[test]
    fn search_issue_maps_to_pull_request() {
        let json = r#"{
            "id": 1, "number": 42, "title": "Fix it", "body": null,
            "user": {"login": "octo", "avatar_url": "https://x"},
            "html_url": "https://github.com/a/b/pull/42",
            "repository_url": "https://api.github.com/repos/a/b",
            "draft": true,
            "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-02T00:00:00Z"
        }"#;
        let issue: SearchIssue = serde_json::from_str(json).unwrap();
        let pull_request = issue.into_pull_request().unwrap();
        assert_eq!(pull_request.repo.full_name(), "a/b");
        assert_eq!(pull_request.number, 42);
        assert!(pull_request.draft);
    }
}
