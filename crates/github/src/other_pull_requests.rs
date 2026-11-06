use serde::de::IgnoredAny;

use crate::github_client::{API_BASE, JSON_MEDIA_TYPE, PAGE_SIZE};
use crate::search_issue::SearchIssue;
use crate::{GitHubClient, GitHubError, PullRequest, RepoRef};

/// Open pull requests by anyone but the viewer, as the stack membership
/// query searches for them; `repo:` qualifiers follow.
const OTHERS_QUERY: &str = "is:pr is:open archived:false -author:@me";
/// Repositories per stack membership search.
const REPOSITORIES_PER_SEARCH: usize = 10;
/// Pages of 100 issues and pull requests followed per repository and
/// refresh (2000 items), so a huge repository cannot stall the refresh.
const MAX_ISSUE_PAGES: usize = 20;
/// An issue's `state` while it is open.
const OPEN_STATE: &str = "open";

/// An item of `GET /repos/{o}/{r}/issues`: an issue or a pull request
/// (issue-shaped, with the issue's id like the search results).
#[derive(serde::Deserialize)]
struct RepositoryIssue {
    #[serde(flatten)]
    issue: SearchIssue,
    state: String,
    /// Present only on pull requests.
    pull_request: Option<IgnoredAny>,
}

/// How a repository's pull requests are brought up to date.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SyncMode {
    /// Nothing known yet: every open one, newest first.
    Full,
    /// Only what changed at or after this time (GitHub's `updated_at` of
    /// the newest one known), open or closed, to merge into what is known.
    Since(String),
}

impl SyncMode {
    /// Full without known pull requests; else since the newest known one.
    fn for_known(known: &[PullRequest]) -> Self {
        known
            .iter()
            .map(|pull_request| pull_request.updated_at.as_str())
            .max()
            .map_or(Self::Full, |newest| Self::Since(newest.to_owned()))
    }
}

impl GitHubClient {
    /// The open pull requests of `repo` that `viewer_login` did not open,
    /// most recently updated first, brought up to date from `known` (what
    /// the last refresh found for it): with nothing known, every open one
    /// (one request per 100 issues and pull requests); otherwise only what
    /// changed since the newest known one, usually one request, merged in:
    /// changed ones replace their old version, closed and merged ones go.
    /// Uses the issues API (core budget, not search) so ids match the
    /// search results of the other lists.
    pub fn other_pull_requests(
        &self,
        repo: &RepoRef,
        known: &[PullRequest],
        viewer_login: &str,
    ) -> Result<Vec<PullRequest>, GitHubError> {
        // Fetch every open one, or every change since the newest known.
        let mode = SyncMode::for_known(known);
        let changed = self.repository_issues(repo, &mode)?;

        // Merge: drop the old version of anything that changed, then add the open ones.
        let mut pull_requests: Vec<PullRequest> = match mode {
            SyncMode::Full => Vec::new(),
            SyncMode::Since(_) => known
                .iter()
                .filter(|pull_request| !changed.iter().any(|item| item.issue.id == pull_request.id))
                .cloned()
                .collect(),
        };
        pull_requests.extend(changed.into_iter().filter_map(|item| {
            let is_others_open_pull_request = item.pull_request.is_some()
                && item.state == OPEN_STATE
                && item.issue.user.login != viewer_login;
            is_others_open_pull_request
                .then(|| item.issue.into_pull_request())
                .flatten()
        }));
        pull_requests.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(pull_requests)
    }

    /// The issues and pull requests of `repo` that `mode` asks for, newest
    /// first, following up to [`MAX_ISSUE_PAGES`] pages.
    fn repository_issues(
        &self,
        repo: &RepoRef,
        mode: &SyncMode,
    ) -> Result<Vec<RepositoryIssue>, GitHubError> {
        let first_page = format!("{API_BASE}/repos/{}/{}/issues", repo.owner, repo.name);
        let mut parameters = vec![
            ("sort", "updated"),
            ("direction", "desc"),
            ("per_page", PAGE_SIZE),
        ];
        match mode {
            SyncMode::Full => parameters.push(("state", "open")),
            SyncMode::Since(since) => {
                parameters.push(("state", "all"));
                parameters.push(("since", since.as_str()));
            }
        }
        let mut items = Vec::new();
        let mut next = Some((first_page, parameters));
        for _ in 0..MAX_ISSUE_PAGES {
            let Some((url, parameters)) = next.take() else {
                break;
            };
            let response = self.get(&url, &parameters, None, JSON_MEDIA_TYPE, &mut |_| {})?;
            items.extend(serde_json::from_str::<Vec<RepositoryIssue>>(
                &response.body,
            )?);
            // The next page's URL already carries its query.
            next = response.next_page.map(|url| (url, Vec::new()));
        }
        Ok(items)
    }
}

/// The search texts for others' pull requests in `repositories`, ten
/// repositories each, for the stack membership query; none without
/// repositories.
pub(crate) fn others_search_queries(repositories: &[RepoRef]) -> Vec<String> {
    repositories
        .chunks(REPOSITORIES_PER_SEARCH)
        .map(|group| {
            group.iter().fold(OTHERS_QUERY.to_owned(), |query, repo| {
                format!("{query} repo:{}", repo.full_name())
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str) -> RepoRef {
        RepoRef {
            owner: "o".into(),
            name: name.into(),
        }
    }

    /// Each search names its repositories, ten at most.
    #[test]
    fn groups_repositories_ten_per_search() {
        let repositories: Vec<RepoRef> = (0..12).map(|index| repo(&format!("r{index}"))).collect();
        let queries = others_search_queries(&repositories);
        assert_eq!(queries.len(), 2);
        assert_eq!(
            queries[1],
            "is:pr is:open archived:false -author:@me repo:o/r10 repo:o/r11"
        );
        assert_eq!(others_search_queries(&[]), Vec::<String>::new());
    }

    /// With nothing known, every open pull request is fetched.
    #[test]
    fn fetches_everything_when_nothing_is_known() {
        assert_eq!(SyncMode::for_known(&[]), SyncMode::Full);
    }

    /// An issues-API item reads as an issue or a pull request with its state.
    #[test]
    fn reads_repository_issues() {
        let json = r#"[
            {"id": 1, "number": 5, "title": "PR", "body": null, "state": "open",
             "user": {"login": "ana", "avatar_url": "https://x"},
             "html_url": "https://github.com/o/r/pull/5",
             "repository_url": "https://api.github.com/repos/o/r",
             "draft": false, "labels": [],
             "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-02T00:00:00Z",
             "pull_request": {"url": "https://api.github.com/repos/o/r/pulls/5"}},
            {"id": 2, "number": 6, "title": "Issue", "body": null, "state": "closed",
             "user": {"login": "bo", "avatar_url": "https://x"},
             "html_url": "https://github.com/o/r/issues/6",
             "repository_url": "https://api.github.com/repos/o/r",
             "labels": [],
             "created_at": "2026-01-01T00:00:00Z", "updated_at": "2026-01-03T00:00:00Z"}
        ]"#;
        let items: Vec<RepositoryIssue> = serde_json::from_str(json).unwrap();
        assert!(items[0].pull_request.is_some() && items[0].state == OPEN_STATE);
        assert!(items[1].pull_request.is_none());
    }
}
