use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepoRef {
    pub owner: String,
    pub name: String,
}

impl RepoRef {
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.owner, self.name)
    }

    /// Parse from a GitHub API repo URL like `https://api.github.com/repos/{owner}/{name}`.
    pub fn from_api_url(url: &str) -> Option<Self> {
        let rest = url.split("/repos/").nth(1)?;
        let mut parts = rest.split('/');
        Some(Self {
            owner: parts.next()?.to_owned(),
            name: parts.next()?.to_owned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub avatar_url: String,
}

/// A pull request as returned by the search API (issue-shaped) and normalized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: u64,
    pub number: u64,
    pub repo: RepoRef,
    pub title: String,
    pub body: Option<String>,
    pub author: User,
    pub html_url: String,
    pub draft: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PullRequestFile {
    pub filename: String,
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    /// Unified diff hunk; absent for binary or very large files.
    pub patch: Option<String>,
}

// --- raw API shapes -------------------------------------------------------

#[derive(Deserialize)]
pub(crate) struct SearchResponse<T> {
    pub items: Vec<T>,
}

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

    #[test]
    fn parses_repo_from_api_url() {
        let repo =
            RepoRef::from_api_url("https://api.github.com/repos/zed-industries/zed").unwrap();
        assert_eq!(repo.full_name(), "zed-industries/zed");
    }

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
        let pr = issue.into_pull_request().unwrap();
        assert_eq!(pr.repo.full_name(), "a/b");
        assert_eq!(pr.number, 42);
        assert!(pr.draft);
    }
}
