use std::collections::HashSet;

use serde::de::IgnoredAny;

use crate::github_client::{JSON_MEDIA_TYPE, PAGE_SIZE};
use crate::pull_request_lookup_query::{PullRequestLookupData, pull_request_lookup_query};
use crate::search_issue::SearchIssue;
use crate::{GitHubClient, GitHubError, OtherPullRequestsUpdate, PullRequest, RepoRef};

/// Pages of 100 issues and pull requests followed per repository and
/// refresh (2000 items), so a huge repository cannot stall the refresh.
const MAX_ISSUE_PAGES: usize = 20;
/// An issue's `state` while it is open.
const OPEN_STATE: &str = "open";
/// Pull requests whose branches and viewer review one GraphQL request
/// looks up (one point of the GraphQL budget each).
const BRANCH_LOOKUPS_PER_REQUEST: usize = 100;

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

/// The items one sync read, newest first, each once.
struct RepositoryIssues {
    items: Vec<RepositoryIssue>,
    /// Whether the pages ran out before [`MAX_ISSUE_PAGES`]: every item
    /// asked for was read.
    is_complete: bool,
}

/// How a repository's pull requests are brought up to date.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SyncMode {
    /// Nothing known yet: every open one, newest first.
    Full,
    /// Only what changed at or after this time (GitHub's `updated_at`),
    /// open or closed, to merge into what is known.
    Since(String),
}

impl SyncMode {
    /// Since the last sync's end when known; else since the newest known
    /// pull request; else Full.
    fn decide(known: &[PullRequest], synced_until: Option<&str>) -> Self {
        let newest_known = known
            .iter()
            .map(|pull_request| pull_request.updated_at.as_str())
            .max();
        synced_until
            .or(newest_known)
            .map_or(Self::Full, |since| Self::Since(since.to_owned()))
    }
}

impl GitHubClient {
    /// The open pull requests of `repo` that `viewer_login` did not open,
    /// most recently updated first, brought up to date from `known` (what
    /// the last refresh found for it) and `synced_until` (where the last
    /// sync of it ended, when this session knows): with neither, every open
    /// one (one request per 100 issues and pull requests); otherwise only
    /// what changed since then, usually one request, merged in: changed ones
    /// replace their old version, closed and merged ones go. When more
    /// changed than 20 pages of 100 hold, it starts over with every open
    /// one, so no change is missed. Uses the issues API (core budget, not
    /// search) so ids match the search results of the other lists. The
    /// branches and the viewer's latest review of every pull request read
    /// anew (the issues API lacks them) or not known yet are looked up with
    /// GraphQL, one request per 100; unchanged ones keep what `known` had
    /// (a review changes `updated_at`, so it is read anew). A renamed or transferred
    /// repository is `GitHubError::Moved`.
    pub fn other_pull_requests(
        &self,
        repo: &RepoRef,
        known: &[PullRequest],
        synced_until: Option<&str>,
        viewer_login: &str,
    ) -> Result<OtherPullRequestsUpdate, GitHubError> {
        // Fetch every change since the last sync; when that did not reach back
        // to its start, every open one instead.
        let mut mode = SyncMode::decide(known, synced_until);
        let mut fetched = self.repository_issues(repo, &mode)?;
        if !fetched.is_complete && mode != SyncMode::Full {
            mode = SyncMode::Full;
            fetched = self.repository_issues(repo, &mode)?;
        }

        // The next sync starts at the newest item read, issue or closed one too.
        let next_synced_until = fetched
            .items
            .iter()
            .map(|item| item.issue.updated_at.as_str())
            .max()
            .map(str::to_owned)
            .or_else(|| match &mode {
                SyncMode::Since(since) => Some(since.clone()),
                SyncMode::Full => None,
            });

        // Merge: drop the old version of anything that changed, then add the open ones.
        let changed = fetched.items;
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

        // The branches and reviews of the ones read anew, which the issues API lacks.
        self.fill_missing_branches(repo, &mut pull_requests)?;
        Ok(OtherPullRequestsUpdate {
            pull_requests,
            synced_until: next_synced_until,
        })
    }

    /// Looks up the branches and the viewer's latest review of
    /// `pull_requests` (all of `repo`) that have no branches yet,
    /// [`BRANCH_LOOKUPS_PER_REQUEST`] per GraphQL request. One GraphQL does
    /// not report keeps none.
    fn fill_missing_branches(
        &self,
        repo: &RepoRef,
        pull_requests: &mut [PullRequest],
    ) -> Result<(), GitHubError> {
        let missing: Vec<u64> = pull_requests
            .iter()
            .filter(|pull_request| pull_request.branches.is_none())
            .map(|pull_request| pull_request.number)
            .collect();
        for numbers in missing.chunks(BRANCH_LOOKUPS_PER_REQUEST) {
            // Ask for this batch, then fill in what came back.
            let data: PullRequestLookupData = self.graphql(
                &pull_request_lookup_query(numbers),
                &serde_json::json!({ "owner": repo.owner, "name": repo.name }),
            )?;
            let mut found = data.into_pull_requests();
            for pull_request in pull_requests.iter_mut() {
                if let Some(looked_up) = found.remove(&pull_request.number) {
                    pull_request.branches = Some(looked_up.branches);
                    pull_request.viewer_review = looked_up.viewer_review;
                }
            }
        }
        Ok(())
    }

    /// The issues and pull requests of `repo` that `mode` asks for, newest
    /// first, each once, following up to [`MAX_ISSUE_PAGES`] pages.
    fn repository_issues(
        &self,
        repo: &RepoRef,
        mode: &SyncMode,
    ) -> Result<RepositoryIssues, GitHubError> {
        let first_page = format!("{}/issues", repo.api_url());
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
        let mut seen_ids: HashSet<u64> = HashSet::new();
        let mut next = Some((first_page, parameters));
        for _ in 0..MAX_ISSUE_PAGES {
            let Some((url, parameters)) = next.take() else {
                break;
            };
            let response = self.get(&url, &parameters, None, JSON_MEDIA_TYPE, &mut |_| {})?;
            let page = serde_json::from_str::<Vec<RepositoryIssue>>(&response.body)?;

            // GitHub answers a renamed repository under its new name; say so instead of listing it apart.
            if let Some(moved_to) = page
                .iter()
                .filter_map(|item| RepoRef::from_api_url(&item.issue.repository_url))
                .find(|actual| actual != repo)
            {
                return Err(GitHubError::Moved {
                    from: repo.full_name(),
                    to: moved_to.full_name(),
                });
            }

            // An item updated while paging moves up, so the next page can repeat one: keep it once.
            items.extend(
                page.into_iter()
                    .filter(|item| seen_ids.insert(item.issue.id)),
            );
            // The next page's URL already carries its query.
            next = response.next_page.map(|url| (url, Vec::new()));
        }
        Ok(RepositoryIssues {
            items,
            is_complete: next.is_none(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With nothing known, every open pull request is fetched; a known end
    /// of the last sync wins over the newest known pull request.
    #[test]
    fn syncs_from_the_last_sync_when_known() {
        assert_eq!(SyncMode::decide(&[], None), SyncMode::Full);
        assert_eq!(
            SyncMode::decide(&[], Some("2026-01-02T00:00:00Z")),
            SyncMode::Since("2026-01-02T00:00:00Z".to_owned())
        );
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
