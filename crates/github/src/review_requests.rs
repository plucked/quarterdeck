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
    /// and the viewer's pending review. Additional pages are read for long conversations and large reviews.
    pub fn review_state(
        &self,
        repo: &RepoRef,
        number: u64,
    ) -> Result<PullRequestReviewState, GitHubError> {
        collect_review_pages(|cursor| {
            let mut data: ReviewStateData = self.graphql(
                REVIEW_STATE_QUERY,
                &serde_json::json!({
                    "owner":repo.owner, "name":repo.name, "number":number,"cursor":cursor
                }),
            )?;
            let next = data.complete_comments(|thread, after| {
                self.graphql(
                    crate::review_queries::THREAD_COMMENTS_QUERY,
                    &serde_json::json!({"thread":thread,"cursor":after}),
                )
            })?;
            Ok((data.into_state().map_err(graphql_error)?, next))
        })
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
        if state.pending_review_id.is_some()
            && state.pending_review_commit.as_deref() != Some(&submission.head_sha)
        {
            return Err(graphql_error("The pending review belongs to another revision. Finish or discard it before submitting this review.".into()));
        }
        let event = submission.event.as_github();
        let immediate = state.pending_review_id.is_none() && submission.replies.is_empty();
        let review_id = if let Some(review_id) = &state.pending_review_id {
            self.add_threads(review_id, submission, on_submitted)?;
            review_id.clone()
        } else {
            let variables =
                new_review_variables(&state.pull_request_node_id, submission, immediate);
            let data: AddReviewData = self.graphql(ADD_REVIEW_MUTATION, &variables)?;
            let review_id = data.into_review_id().map_err(graphql_error)?;
            for index in 0..submission.threads.len() {
                on_submitted(SubmittedPart::Thread(index));
            }
            if immediate {
                return Ok(());
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

fn new_review_variables(
    pr: &str,
    submission: &ReviewSubmission,
    immediate: bool,
) -> serde_json::Value {
    let threads: Vec<_> = submission.threads.iter().map(draft_thread).collect();
    let mut variables =
        serde_json::json!({"pullRequestId":pr,"commit":submission.head_sha,"threads":threads});
    if immediate {
        variables["event"] = submission.event.as_github().into();
        variables["body"] = submission.body.as_str().into();
    }
    variables
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

fn collect_review_pages(
    mut page: impl FnMut(Option<&str>) -> Result<(PullRequestReviewState, Option<String>), GitHubError>,
) -> Result<PullRequestReviewState, GitHubError> {
    let (mut state, mut next) = page(None)?;
    let mut seen = std::collections::HashSet::new();
    while let Some(cursor) = next {
        if !seen.insert(cursor.clone()) {
            return Err(graphql_error("GitHub repeated a thread cursor".into()));
        }
        let page = page(Some(&cursor))?;
        state.threads.extend(page.0.threads);
        next = page.1;
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReviewEvent;
    fn state() -> PullRequestReviewState {
        PullRequestReviewState {
            pull_request_node_id: "PR".into(),
            viewer_login: "reader".into(),
            viewer_is_author: false,
            pending_review_id: None,
            pending_review_commit: None,
            threads: vec![],
        }
    }
    #[test]
    fn every_new_review_binds_the_displayed_head_and_pending_mismatch_is_rejected() {
        let submission = ReviewSubmission {
            head_sha: "shown".into(),
            event: ReviewEvent::Approve,
            body: "Looks good".into(),
            threads: vec![],
            replies: vec![],
        };
        for immediate in [true, false] {
            let variables = new_review_variables("PR", &submission, immediate);
            assert_eq!(variables["commit"], "shown");
            assert_eq!(variables.get("event").is_some(), immediate);
        }
        assert!(ADD_REVIEW_MUTATION.contains("commitOID: $commit"));
        let mut pending = state();
        pending.pending_review_id = Some("R".into());
        for commit in [None, Some("other".into())] {
            pending.pending_review_commit = commit;
            let error = GitHubClient::new("unused")
                .submit_review(&pending, &submission, &mut |_| {
                    panic!("published a mismatched draft")
                })
                .unwrap_err();
            assert!(error.to_string().contains("another revision"));
        }
    }

    #[test]
    fn follows_thread_pages_until_complete_and_rejects_repeated_cursors() {
        let mut visits = Vec::new();
        collect_review_pages(|cursor| {
            visits.push(cursor.map(str::to_owned));
            Ok((state(), cursor.is_none().then(|| "page2".into())))
        })
        .unwrap();
        assert_eq!(visits, [None, Some("page2".into())]);
        assert!(collect_review_pages(|_| Ok((state(), Some("stuck".into())))).is_err());
    }
}
