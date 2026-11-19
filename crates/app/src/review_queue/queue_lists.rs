use qd_github::{PullRequest, RepoRef};

/// The queue's lists: the viewer's own pull requests, those that request
/// the viewer's review, others' the viewer has reviewed (GitHub takes the
/// viewer off the requested reviewers once a review is in), and others' in
/// the watched repositories (before their filters). A pull request can be
/// in more than one.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct QueueLists {
    pub authored: Vec<PullRequest>,
    pub review_requested: Vec<PullRequest>,
    pub reviewed: Vec<PullRequest>,
    pub others: Vec<PullRequest>,
}

impl QueueLists {
    /// Current direct review requests, including requests to review a PR again.
    pub fn review_tab(&self) -> &[PullRequest] {
        &self.review_requested
    }

    /// Every listed pull request: authored, review requested, reviewed, then others.
    pub fn iter(&self) -> impl Iterator<Item = &PullRequest> {
        self.authored
            .iter()
            .chain(&self.review_requested)
            .chain(&self.reviewed)
            .chain(&self.others)
    }

    /// The listed pull request with GitHub id `id`.
    pub fn find_by_id(&self, id: u64) -> Option<&PullRequest> {
        self.iter().find(|pull_request| pull_request.id == id)
    }

    /// The listed pull request `number` of `repo`.
    pub fn find_by_number(&self, repo: &RepoRef, number: u64) -> Option<&PullRequest> {
        self.iter()
            .find(|pull_request| pull_request.number == number && &pull_request.repo == repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qd_github::User;

    fn pull_request(id: u64, updated_at: &str) -> PullRequest {
        PullRequest {
            id,
            number: id,
            repo: RepoRef {
                owner: "o".into(),
                name: "r".into(),
            },
            title: String::new(),
            body: None,
            author: User {
                login: "octo".into(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft: false,
            created_at: String::new(),
            updated_at: updated_at.into(),
            labels: Vec::new(),
            stack: None,
            branches: None,
            viewer_review: None,
        }
    }

    /// A completed review leaves the tab until GitHub requests another review.
    #[test]
    fn review_tab_follows_current_requests() {
        let mut reviewed = pull_request(1, "2026-11-03");
        reviewed.viewer_review = Some(qd_github::ViewerReview::Approved);
        let first_request = pull_request(2, "2026-11-01");
        let mut lists = QueueLists {
            review_requested: vec![first_request.clone()],
            reviewed: vec![reviewed.clone()],
            ..QueueLists::default()
        };
        assert_eq!(lists.review_tab(), std::slice::from_ref(&first_request));

        lists.review_requested.insert(0, reviewed.clone());
        assert_eq!(lists.review_tab(), [reviewed, first_request]);
    }
}
