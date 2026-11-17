//! The GraphQL texts of reviewing and the shapes of their answers.

use serde::Deserialize;

use crate::{LineSide, PullRequestReviewState, ReviewThread, ThreadComment, ThreadPermissions};

/// The review state of one pull request: threads with their comments, the
/// viewer, and the viewer's pending review (only its author sees one).
pub(crate) const REVIEW_STATE_QUERY: &str = r"
query ReviewState($owner: String!, $name: String!, $number: Int!, $cursor: String) {
  viewer { login }
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      id
      viewerDidAuthor
      reviews(states: PENDING, first: 1) { nodes { id commit { oid } } }
      reviewThreads(first: 100, after: $cursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id path line startLine originalLine diffSide isResolved isOutdated
          viewerCanReply viewerCanResolve viewerCanUnresolve
          comments(first: 100) {
            pageInfo { hasNextPage endCursor }
            nodes {
              id body createdAt viewerCanUpdate viewerCanDelete
              author { login avatarUrl }
              pullRequestReview { state }
            }
          }
        }
      }
    }
  }
}
";

pub(crate) const THREAD_COMMENTS_QUERY: &str = r"
query ThreadComments($thread: ID!, $cursor: String!) {
  node(id: $thread) { ... on PullRequestReviewThread {
    comments(first: 100, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
              id body createdAt viewerCanUpdate viewerCanDelete
              author { login avatarUrl }
              pullRequestReview { state }
      }
    }
  } }
}
";

/// Creates the viewer's review with new threads; with an event it is
/// submitted at once, without one it stays pending.
pub(crate) const ADD_REVIEW_MUTATION: &str = r"
mutation AddReview($pullRequestId: ID!, $commit: GitObjectID!, $event: PullRequestReviewEvent, $body: String, $threads: [DraftPullRequestReviewThread]) {
  addPullRequestReview(input: {pullRequestId: $pullRequestId, commitOID: $commit, event: $event, body: $body, threads: $threads}) {
    pullRequestReview { id }
  }
}
";

/// Adds a thread to a pending review.
pub(crate) const ADD_THREAD_MUTATION: &str = r"
mutation AddThread($reviewId: ID!, $path: String!, $body: String!, $line: Int!, $side: DiffSide!, $startLine: Int, $startSide: DiffSide) {
  addPullRequestReviewThread(input: {pullRequestReviewId: $reviewId, path: $path, body: $body, line: $line, side: $side, startLine: $startLine, startSide: $startSide}) {
    thread { id }
  }
}
";

/// Adds a reply to a thread, inside a pending review.
pub(crate) const ADD_REPLY_MUTATION: &str = r"
mutation AddReply($reviewId: ID!, $threadId: ID!, $body: String!) {
  addPullRequestReviewThreadReply(input: {pullRequestReviewId: $reviewId, pullRequestReviewThreadId: $threadId, body: $body}) {
    comment { id }
  }
}
";

/// Submits a pending review with its verdict and summary.
pub(crate) const SUBMIT_REVIEW_MUTATION: &str = r"
mutation SubmitReview($reviewId: ID!, $event: PullRequestReviewEvent!, $body: String) {
  submitPullRequestReview(input: {pullRequestReviewId: $reviewId, event: $event, body: $body}) {
    pullRequestReview { id }
  }
}
";

/// Discards a pending review and its comments.
pub(crate) const DELETE_REVIEW_MUTATION: &str = r"
mutation DeleteReview($reviewId: ID!) {
  deletePullRequestReview(input: {pullRequestReviewId: $reviewId}) { pullRequestReview { id } }
}
";

/// Changes a comment's text.
pub(crate) const UPDATE_COMMENT_MUTATION: &str = r"
mutation UpdateComment($id: ID!, $body: String!) {
  updatePullRequestReviewComment(input: {pullRequestReviewCommentId: $id, body: $body}) {
    pullRequestReviewComment { id }
  }
}
";

/// Deletes a comment.
pub(crate) const DELETE_COMMENT_MUTATION: &str = r"
mutation DeleteComment($id: ID!) {
  deletePullRequestReviewComment(input: {id: $id}) { pullRequestReview { id } }
}
";

/// Marks a thread resolved.
pub(crate) const RESOLVE_THREAD_MUTATION: &str = r"
mutation ResolveThread($threadId: ID!) {
  resolveReviewThread(input: {threadId: $threadId}) { thread { id } }
}
";

/// Marks a thread unresolved.
pub(crate) const UNRESOLVE_THREAD_MUTATION: &str = r"
mutation UnresolveThread($threadId: ID!) {
  unresolveReviewThread(input: {threadId: $threadId}) { thread { id } }
}
";

/// The data of [`REVIEW_STATE_QUERY`].
#[derive(Deserialize)]
pub(crate) struct ReviewStateData {
    viewer: Login,
    repository: Option<RepositoryNode>,
}

#[derive(Deserialize)]
struct Login {
    login: String,
}

/// A comment's author: the login and the avatar shown beside it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentAuthor {
    login: String,
    #[serde(default)]
    avatar_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryNode {
    pull_request: Option<PullRequestNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestNode {
    id: String,
    viewer_did_author: bool,
    reviews: Nodes<PendingReview>,
    review_threads: Nodes<ThreadNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Nodes<T> {
    page_info: Option<PageInfo>,
    nodes: Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

impl<T> Nodes<T> {
    fn next(&self) -> Result<Option<String>, String> {
        if let Some(info) = &self.page_info
            && info.has_next_page
        {
            return info
                .end_cursor
                .clone()
                .map(Some)
                .ok_or_else(|| "GitHub omitted the next page cursor".into());
        }
        Ok(None)
    }
}

#[derive(Deserialize)]
pub(crate) struct ThreadPage {
    node: Option<CommentPage>,
}

#[derive(Deserialize)]
struct CommentPage {
    comments: Nodes<CommentNode>,
}

#[derive(Deserialize)]
struct PendingReview {
    id: String,
    commit: Option<Commit>,
}

#[derive(Deserialize)]
struct Commit {
    oid: String,
}

#[derive(Deserialize)]
struct IdNode {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThreadNode {
    id: String,
    path: String,
    line: Option<u32>,
    start_line: Option<u32>,
    original_line: Option<u32>,
    diff_side: String,
    is_resolved: bool,
    is_outdated: bool,
    #[serde(flatten)]
    permissions: PermissionsNode,
    comments: Nodes<CommentNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PermissionsNode {
    viewer_can_reply: bool,
    viewer_can_resolve: bool,
    viewer_can_unresolve: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommentNode {
    id: String,
    body: String,
    created_at: String,
    viewer_can_update: bool,
    viewer_can_delete: bool,
    author: Option<CommentAuthor>,
    pull_request_review: Option<ReviewNode>,
}

#[derive(Deserialize)]
struct ReviewNode {
    state: String,
}

impl ReviewStateData {
    pub(crate) fn complete_comments(
        &mut self,
        mut fetch: impl FnMut(&str, &str) -> Result<ThreadPage, crate::GitHubError>,
    ) -> Result<Option<String>, crate::GitHubError> {
        use crate::github_client::graphql_error;
        let pr = self
            .repository
            .as_mut()
            .and_then(|r| r.pull_request.as_mut())
            .ok_or_else(|| graphql_error("GitHub knows no such pull request".into()))?;
        for thread in &mut pr.review_threads.nodes {
            let mut cursors = std::collections::HashSet::new();
            while let Some(cursor) = thread.comments.next().map_err(graphql_error)? {
                if !cursors.insert(cursor.clone()) {
                    return Err(graphql_error("GitHub repeated a comment cursor".into()));
                }
                let page = fetch(&thread.id, &cursor)?
                    .node
                    .ok_or_else(|| graphql_error("Review thread changed during loading".into()))?;
                thread.comments.nodes.extend(page.comments.nodes);
                thread.comments.page_info = page.comments.page_info;
            }
        }
        pr.review_threads.next().map_err(graphql_error)
    }

    /// The review state, or why there is none (no such pull request).
    pub fn into_state(self) -> Result<PullRequestReviewState, String> {
        let pull_request = self
            .repository
            .and_then(|repository| repository.pull_request)
            .ok_or("GitHub knows no such pull request")?;
        Ok(PullRequestReviewState {
            pull_request_node_id: pull_request.id,
            viewer_login: self.viewer.login,
            viewer_is_author: pull_request.viewer_did_author,
            pending_review_commit: pull_request
                .reviews
                .nodes
                .first()
                .and_then(|r| r.commit.as_ref())
                .map(|commit| commit.oid.clone()),
            pending_review_id: pull_request
                .reviews
                .nodes
                .into_iter()
                .next()
                .map(|review| review.id),
            threads: pull_request
                .review_threads
                .nodes
                .into_iter()
                .map(ThreadNode::into_thread)
                .collect(),
        })
    }
}

impl ThreadNode {
    fn into_thread(self) -> ReviewThread {
        ReviewThread {
            id: self.id,
            path: self.path,
            side: LineSide::from_github(&self.diff_side),
            line: self.line,
            start_line: self.start_line,
            original_line: self.original_line,
            is_resolved: self.is_resolved,
            is_outdated: self.is_outdated,
            permissions: ThreadPermissions {
                can_reply: self.permissions.viewer_can_reply,
                can_resolve: self.permissions.viewer_can_resolve,
                can_unresolve: self.permissions.viewer_can_unresolve,
            },
            comments: self
                .comments
                .nodes
                .into_iter()
                .map(|comment| {
                    let (author_login, author_avatar_url) = comment
                        .author
                        .map(|author| (author.login, author.avatar_url))
                        .unwrap_or_default();
                    ThreadComment {
                        id: comment.id,
                        author_login,
                        author_avatar_url,
                        body: comment.body,
                        created_at: comment.created_at,
                        is_pending: comment
                            .pull_request_review
                            .is_some_and(|review| review.state == "PENDING"),
                        can_update: comment.viewer_can_update,
                        can_delete: comment.viewer_can_delete,
                    }
                })
                .collect(),
        }
    }
}

/// The answer of [`ADD_REVIEW_MUTATION`].
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddReviewData {
    add_pull_request_review: Option<ReviewIdPayload>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewIdPayload {
    pull_request_review: Option<IdNode>,
}

impl AddReviewData {
    /// The new review's id.
    pub fn into_review_id(self) -> Result<String, String> {
        self.add_pull_request_review
            .and_then(|payload| payload.pull_request_review)
            .map(|review| review.id)
            .ok_or_else(|| "GitHub created no review".to_owned())
    }
}

/// Any mutation answer whose content is not read.
#[derive(Deserialize)]
pub(crate) struct IgnoredData {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphql_response::GraphQlResponse;

    /// Threads read with their side, lines, permissions and pending comments.
    #[test]
    fn reads_review_state() {
        let json = r#"{"data": {
            "viewer": {"login": "me"},
            "repository": {"pullRequest": {
                "id": "PR_1", "viewerDidAuthor": false,
                "reviews": {"nodes": [{"id": "R_1","commit":{"oid":"original"}}]},
                "reviewThreads": {"nodes": [{
                    "id": "T_1", "path": "src/a.rs", "line": 12, "startLine": null,
                    "originalLine": 10, "diffSide": "LEFT", "isResolved": false, "isOutdated": false,
                    "viewerCanReply": true, "viewerCanResolve": true, "viewerCanUnresolve": false,
                    "comments": {"nodes": [{
                        "id": "C_1", "body": "Why?", "createdAt": "2026-01-01T00:00:00Z",
                        "viewerCanUpdate": true, "viewerCanDelete": true,
                        "author": {"login": "me"}, "pullRequestReview": {"state": "PENDING"}
                    }]}
                }]}
            }}
        }}"#;
        let mut json: serde_json::Value = serde_json::from_str(json).unwrap();
        let comments =
            &mut json["data"]["repository"]["pullRequest"]["reviewThreads"]["nodes"][0]["comments"];
        comments["pageInfo"] = serde_json::json!({"hasNextPage":true,"endCursor":"next"});
        let mut second = comments["nodes"][0].clone();
        second["id"] = "C_2".into();
        let response: GraphQlResponse<ReviewStateData> = serde_json::from_value(json).unwrap();
        let mut data = response.into_data().unwrap();
        let next=data.complete_comments(|thread,cursor|{
            assert_eq!((thread,cursor),("T_1","next"));
            Ok(serde_json::from_value(serde_json::json!({"node":{"comments":{"nodes":[second],"pageInfo":{"hasNextPage":false,"endCursor":null}}}}))?)
        }).unwrap();
        assert!(next.is_none());
        let state = data.into_state().unwrap();
        assert_eq!(state.threads[0].comments[1].id, "C_2");
        assert_eq!(state.pending_review_id.as_deref(), Some("R_1"));
        assert_eq!(state.pending_review_commit.as_deref(), Some("original"));
        let thread = &state.threads[0];
        assert_eq!((thread.side, thread.line), (LineSide::Old, Some(12)));
        assert!(thread.comments[0].is_pending);
    }
}
