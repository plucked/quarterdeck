use crate::github_client::graphql_error;
use crate::review_queries::{
    ADD_REPLY_MUTATION, ADD_REVIEW_MUTATION, ADD_THREAD_MUTATION, AddReviewData,
    DELETE_COMMENT_MUTATION, DELETE_REVIEW_MUTATION, IgnoredData, RESOLVE_THREAD_MUTATION,
    REVIEW_STATE_QUERY, ReviewStateData, SUBMIT_REVIEW_MUTATION, UNRESOLVE_THREAD_MUTATION,
    UPDATE_COMMENT_MUTATION,
};
use crate::{
    GitHubClient, GitHubError, NewReviewThread, PullRequestReviewState, RepoRef, ReviewSubmission,
    SubmittedPart,
};

impl GitHubClient {
    /// The review state of a pull request: its conversations, the viewer
    /// and the viewer's pending review. One GraphQL request (100 threads at
    /// most, each with 100 comments at most).
    pub fn review_state(
        &self,
        repo: &RepoRef,
        number: u64,
    ) -> Result<PullRequestReviewState, GitHubError> {
        let data: ReviewStateData = self.graphql(
            REVIEW_STATE_QUERY,
            &serde_json::json!({ "owner": repo.owner, "name": repo.name, "number": number }),
        )?;
        data.into_state().map_err(graphql_error)
    }

    /// Submits `submission` as the viewer's review. Without replies or a
    /// pending review it is one request; otherwise the threads and replies
    /// go into the pending review (created first when there is none) one by
    /// one, reported through `on_submitted` as GitHub takes them, then the
    /// review is submitted.
    pub fn submit_review(
        &self,
        state: &PullRequestReviewState,
        submission: &ReviewSubmission,
        on_submitted: &mut dyn FnMut(SubmittedPart),
    ) -> Result<(), GitHubError> {
        let event = submission.event.as_github();
        let threads: Vec<serde_json::Value> = submission.threads.iter().map(draft_thread).collect();

        // Nothing pending and no replies: create and submit in one go.
        if state.pending_review_id.is_none() && submission.replies.is_empty() {
            let data: AddReviewData = self.graphql(
                ADD_REVIEW_MUTATION,
                &serde_json::json!({
                    "pullRequestId": state.pull_request_node_id,
                    "event": event,
                    "body": submission.body,
                    "threads": threads,
                }),
            )?;
            data.into_review_id().map_err(graphql_error)?;
            for index in 0..submission.threads.len() {
                on_submitted(SubmittedPart::Thread(index));
            }
            return Ok(());
        }

        // The pending review: the one begun before, or a new one holding the new threads.
        let review_id = if let Some(review_id) = &state.pending_review_id {
            self.add_threads(review_id, submission, on_submitted)?;
            review_id.clone()
        } else {
            let data: AddReviewData = self.graphql(
                ADD_REVIEW_MUTATION,
                &serde_json::json!({
                    "pullRequestId": state.pull_request_node_id,
                    "threads": threads,
                }),
            )?;
            let review_id = data.into_review_id().map_err(graphql_error)?;
            for index in 0..submission.threads.len() {
                on_submitted(SubmittedPart::Thread(index));
            }
            review_id
        };

        // The replies, then the verdict.
        for (index, reply) in submission.replies.iter().enumerate() {
            let _: IgnoredData = self.graphql(
                ADD_REPLY_MUTATION,
                &serde_json::json!({
                    "reviewId": review_id,
                    "threadId": reply.thread_id,
                    "body": reply.body,
                }),
            )?;
            on_submitted(SubmittedPart::Reply(index));
        }
        let _: IgnoredData = self.graphql(
            SUBMIT_REVIEW_MUTATION,
            &serde_json::json!({ "reviewId": review_id, "event": event, "body": submission.body }),
        )?;
        Ok(())
    }

    /// Adds the submission's threads to pending review `review_id`, one by one.
    fn add_threads(
        &self,
        review_id: &str,
        submission: &ReviewSubmission,
        on_submitted: &mut dyn FnMut(SubmittedPart),
    ) -> Result<(), GitHubError> {
        for (index, thread) in submission.threads.iter().enumerate() {
            let _: IgnoredData = self.graphql(
                ADD_THREAD_MUTATION,
                &serde_json::json!({
                    "reviewId": review_id,
                    "path": thread.path,
                    "body": thread.body,
                    "line": thread.line,
                    "side": thread.side.as_github(),
                    "startLine": thread.start_line,
                    "startSide": thread.start_line.map(|_| thread.side.as_github()),
                }),
            )?;
            on_submitted(SubmittedPart::Thread(index));
        }
        Ok(())
    }

    /// Discards the viewer's pending review with its comments.
    pub fn discard_pending_review(&self, review_id: &str) -> Result<(), GitHubError> {
        let _: IgnoredData = self.graphql(
            DELETE_REVIEW_MUTATION,
            &serde_json::json!({ "reviewId": review_id }),
        )?;
        Ok(())
    }

    /// Changes the text of a review comment the viewer may edit.
    pub fn update_review_comment(&self, comment_id: &str, body: &str) -> Result<(), GitHubError> {
        let _: IgnoredData = self.graphql(
            UPDATE_COMMENT_MUTATION,
            &serde_json::json!({ "id": comment_id, "body": body }),
        )?;
        Ok(())
    }

    /// Deletes a review comment the viewer may delete.
    pub fn delete_review_comment(&self, comment_id: &str) -> Result<(), GitHubError> {
        let _: IgnoredData = self.graphql(
            DELETE_COMMENT_MUTATION,
            &serde_json::json!({ "id": comment_id }),
        )?;
        Ok(())
    }

    /// Marks a conversation resolved (`is_resolved`), or unresolved again.
    pub fn set_thread_resolved(
        &self,
        thread_id: &str,
        is_resolved: bool,
    ) -> Result<(), GitHubError> {
        let mutation = if is_resolved {
            RESOLVE_THREAD_MUTATION
        } else {
            UNRESOLVE_THREAD_MUTATION
        };
        let _: IgnoredData =
            self.graphql(mutation, &serde_json::json!({ "threadId": thread_id }))?;
        Ok(())
    }
}

/// A new thread as GraphQL's `DraftPullRequestReviewThread`.
fn draft_thread(thread: &NewReviewThread) -> serde_json::Value {
    serde_json::json!({
        "path": thread.path,
        "body": thread.body,
        "line": thread.line,
        "side": thread.side.as_github(),
        "startLine": thread.start_line,
        "startSide": thread.start_line.map(|_| thread.side.as_github()),
    })
}
