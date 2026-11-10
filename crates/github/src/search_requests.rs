use crate::github_client::{API_BASE, PAGE_SIZE};
use crate::listed_details_query::{LISTED_DETAILS_QUERY, ListedDetailsData};
use crate::search_issue::SearchIssue;
use crate::search_response::SearchResponse;
use crate::{GitHubClient, GitHubError, ListedPullRequestDetails, PullRequest, RepoRef};

const REVIEW_REQUESTED_QUERY: &str = "is:pr is:open archived:false review-requested:@me";
const AUTHORED_QUERY: &str = "is:pr is:open archived:false author:@me";
/// Open pull requests by others that the viewer submitted a review on.
const REVIEWED_QUERY: &str = "is:pr is:open archived:false reviewed-by:@me -author:@me";
/// Open pull requests by anyone but the viewer, most recently updated
/// first, as the stack membership query searches for them (GraphQL search
/// takes its order from the text); `repo:` qualifiers follow.
const OTHERS_QUERY: &str = "is:pr is:open archived:false -author:@me sort:updated-desc";
/// Repositories the stack membership query searches others' pull requests in.
const REPOSITORIES_PER_SEARCH: usize = 10;

impl GitHubClient {
    /// Open pull requests that request the viewer's review, most recently
    /// updated first. One search request (search has no conditional requests).
    pub fn review_requested(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(REVIEW_REQUESTED_QUERY)
    }

    /// Others' open pull requests the viewer has reviewed (GitHub takes the
    /// viewer off the requested reviewers once a review is submitted), most
    /// recently updated first; may overlap [`Self::review_requested`]. One
    /// search request.
    pub fn reviewed_by_viewer(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(REVIEWED_QUERY)
    }

    /// The viewer's own open pull requests, most recently updated first. One
    /// search request.
    pub fn authored(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(AUTHORED_QUERY)
    }

    /// The branches of the pull requests in the lists, the viewer's latest
    /// review and where they sit in their stacks: the viewer's, those
    /// requesting the viewer's review, those the viewer reviewed, and others' in `other_repositories` (the 100 most recently updated of
    /// the first ten repositories). One GraphQL request (the search API
    /// lacks both).
    pub fn listed_details(
        &self,
        other_repositories: &[RepoRef],
    ) -> Result<Vec<ListedPullRequestDetails>, GitHubError> {
        let others = others_search_query(other_repositories);
        let data: ListedDetailsData = self.graphql(
            LISTED_DETAILS_QUERY,
            &serde_json::json!({
                "authored": AUTHORED_QUERY,
                "reviewRequested": REVIEW_REQUESTED_QUERY,
                "reviewed": REVIEWED_QUERY,
                "includeOthers": others.is_some(),
                "others": others.unwrap_or_default(),
            }),
        )?;
        Ok(data.into_details())
    }

    /// The pull requests a search query finds, most recently updated first.
    fn search_pull_requests(&self, query: &str) -> Result<Vec<PullRequest>, GitHubError> {
        let response: SearchResponse<SearchIssue> = self.get_json(
            &format!("{API_BASE}/search/issues"),
            &[
                ("q", query),
                ("sort", "updated"),
                ("order", "desc"),
                ("per_page", PAGE_SIZE),
            ],
        )?;
        Ok(response
            .items
            .into_iter()
            .filter_map(SearchIssue::into_pull_request)
            .collect())
    }
}

/// The search text for others' pull requests in the first ten of
/// `repositories`, for the stack membership query; none without
/// repositories.
fn others_search_query(repositories: &[RepoRef]) -> Option<String> {
    (!repositories.is_empty()).then(|| {
        repositories
            .iter()
            .take(REPOSITORIES_PER_SEARCH)
            .fold(OTHERS_QUERY.to_owned(), |query, repo| {
                format!("{query} repo:{}", repo.full_name())
            })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The others' search names the first ten repositories; none without any.
    #[test]
    fn others_search_names_the_first_ten_repositories() {
        let repositories: Vec<RepoRef> = (0..12)
            .map(|index| RepoRef {
                owner: "o".into(),
                name: format!("r{index}"),
            })
            .collect();
        let query = others_search_query(&repositories).unwrap();
        assert!(query.starts_with(
            "is:pr is:open archived:false -author:@me sort:updated-desc repo:o/r0 repo:o/r1 "
        ));
        assert!(query.ends_with(" repo:o/r9"));
        assert_eq!(others_search_query(&[]), None);
    }
}
