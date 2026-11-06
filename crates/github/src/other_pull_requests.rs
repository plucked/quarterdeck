use crate::github_client::{API_BASE, JSON_MEDIA_TYPE, PAGE_SIZE};
use crate::search_issue::SearchIssue;
use crate::search_response::SearchResponse;
use crate::{GitHubClient, GitHubError, PullRequest, RepoRef};

/// Open pull requests by anyone but the viewer; `repo:` qualifiers follow.
const OTHERS_QUERY: &str = "is:pr is:open archived:false -author:@me";
/// Repositories per search: GitHub ORs `repo:` qualifiers, and a few per
/// query keeps the URL short.
const REPOSITORIES_PER_SEARCH: usize = 10;
/// Pages followed per search (100 pull requests each).
const MAX_SEARCH_PAGES: usize = 5;

impl GitHubClient {
    /// Open pull requests in `repositories` the viewer did not open, most
    /// recently updated first: one search request per 10 repositories and
    /// page (at most 500 pull requests per 10 repositories).
    pub fn other_pull_requests(
        &self,
        repositories: &[RepoRef],
    ) -> Result<Vec<PullRequest>, GitHubError> {
        let mut pull_requests = Vec::new();
        for query in others_search_queries(repositories) {
            pull_requests.extend(self.search_all_pages(&query)?);
        }
        // Searches over several groups are each sorted; merge them by update time.
        pull_requests.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(pull_requests)
    }

    /// Every page of a search (up to [`MAX_SEARCH_PAGES`]), most recently updated first.
    fn search_all_pages(&self, query: &str) -> Result<Vec<PullRequest>, GitHubError> {
        let mut pull_requests = Vec::new();
        let first_page = format!("{API_BASE}/search/issues");
        let mut next = Some((
            first_page,
            vec![
                ("q", query),
                ("sort", "updated"),
                ("order", "desc"),
                ("per_page", PAGE_SIZE),
            ],
        ));
        for _ in 0..MAX_SEARCH_PAGES {
            let Some((url, parameters)) = next.take() else {
                break;
            };
            let response = self.get(&url, &parameters, None, JSON_MEDIA_TYPE, &mut |_| {})?;
            let page: SearchResponse<SearchIssue> = serde_json::from_str(&response.body)?;
            pull_requests.extend(
                page.items
                    .into_iter()
                    .filter_map(SearchIssue::into_pull_request),
            );
            // The next page's URL already carries its query.
            next = response.next_page.map(|url| (url, Vec::new()));
        }
        Ok(pull_requests)
    }
}

/// The search texts for others' pull requests in `repositories`, ten
/// repositories each; none without repositories.
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
}
