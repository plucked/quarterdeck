use crate::github_client::{API_BASE, PAGE_SIZE};
use crate::search_issue::SearchIssue;
use crate::search_response::SearchResponse;
use crate::{GitHubClient, GitHubError, PullRequest};

const REVIEW_REQUESTED_QUERY: &str = "is:pr is:open archived:false user-review-requested:@me";
const AUTHORED_QUERY: &str = "is:pr is:open archived:false author:@me";
/// Open pull requests by others that the viewer submitted a review on.
const REVIEWED_QUERY: &str = "is:pr is:open archived:false reviewed-by:@me -author:@me";
impl GitHubClient {
    /// Open pull requests that directly request the viewer's review, most recently
    /// updated first. Follows every result page.
    pub fn review_requested(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(REVIEW_REQUESTED_QUERY)
    }

    /// Others' open pull requests the viewer has reviewed (GitHub takes the
    /// viewer off the requested reviewers once a review is submitted), most
    /// recently updated first; may overlap [`Self::review_requested`]. Follows every result page.
    pub fn reviewed_by_viewer(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(REVIEWED_QUERY)
    }

    /// The viewer's own open pull requests, most recently updated first. Follows every result page.
    pub fn authored(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(AUTHORED_QUERY)
    }

    /// The pull requests a search query finds, most recently updated first.
    fn search_pull_requests(&self, query: &str) -> Result<Vec<PullRequest>, GitHubError> {
        let mut params = vec![
            ("q", query),
            ("sort", "updated"),
            ("order", "desc"),
            ("per_page", PAGE_SIZE),
        ];
        collect_search_pages(query, |url| {
            let response = self.get(
                url,
                &params,
                None,
                crate::github_client::JSON_MEDIA_TYPE,
                &mut |_| {},
            )?;
            params.clear();
            Ok(response)
        })
    }
}

fn collect_search_pages(
    query: &str,
    mut fetch: impl FnMut(&str) -> Result<crate::http_response::HttpResponse, GitHubError>,
) -> Result<Vec<PullRequest>, GitHubError> {
    let mut results = Vec::new();
    let mut ids = std::collections::HashSet::new();
    let mut next = Some(format!("{API_BASE}/search/issues"));
    for _ in 0..10 {
        let Some(url) = next.take() else {
            break;
        };
        let response = fetch(&url)?;
        let page: SearchResponse<SearchIssue> = serde_json::from_str(&response.body)?;
        if page.incomplete_results || page.total_count > 1000 {
            return Err(crate::github_client::graphql_error(format!(
                "GitHub search returned incomplete results for {query} ({} matches). The previous queue was kept.",
                page.total_count
            )));
        }
        results.extend(
            page.items
                .into_iter()
                .filter_map(SearchIssue::into_pull_request)
                .filter(|pr| ids.insert(pr.id)),
        );

        next = response.next_page;
    }
    if next.is_some() {
        return Err(crate::github_client::graphql_error(
            "GitHub search exceeded its page limit".into(),
        ));
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(
        id: u64,
        total: usize,
        incomplete: bool,
        next: Option<&str>,
    ) -> crate::http_response::HttpResponse {
        let body=serde_json::json!({"total_count":total,"incomplete_results":incomplete,"items":[{
            "id":id,"number":id,"title":"PR","user":{"login":"a","avatar_url":""},"html_url":"",
            "repository_url":"https://api.github.com/repos/o/r","created_at":"a","updated_at":"b"
        }]}).to_string();
        crate::http_response::HttpResponse {
            status: ureq::http::StatusCode::OK,
            body,
            etag: None,
            next_page: next.map(str::to_owned),
        }
    }
    #[test]
    fn loads_later_queue_rows_and_rejects_incomplete_searches() {
        let found = collect_search_pages("test", |url| {
            Ok(if url == "second" {
                page(2, 2, false, None)
            } else {
                page(1, 2, false, Some("second"))
            })
        })
        .unwrap();
        assert_eq!(found.iter().map(|pr| pr.id).collect::<Vec<_>>(), [1, 2]);
        for (total, partial) in [(1001, false), (1, true)] {
            assert!(collect_search_pages("test", |_| Ok(page(1, total, partial, None))).is_err());
        }
    }
}
