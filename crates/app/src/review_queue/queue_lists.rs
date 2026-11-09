use std::collections::HashSet;

use qd_github::{PullRequest, RepoRef};

/// The queue's lists: the viewer's own pull requests, those that request
/// the viewer's review, others' the viewer has reviewed (GitHub takes the
/// viewer off the requested reviewers once a review is in), and others' in
/// the watched repositories (before their filters). A pull request can be
/// in more than one.
#[derive(Debug, Default)]
pub struct QueueLists {
    pub authored: Vec<PullRequest>,
    pub review_requested: Vec<PullRequest>,
    pub reviewed: Vec<PullRequest>,
    pub others: Vec<PullRequest>,
}

impl QueueLists {
    /// What the Review tab lists: those requesting the viewer's review and
    /// those the viewer reviewed, each once, most recently updated first.
    pub fn review_tab(&self) -> Vec<PullRequest> {
        let mut seen: HashSet<u64> = HashSet::new();
        let mut merged: Vec<PullRequest> = self
            .review_requested
            .iter()
            .chain(&self.reviewed)
            .filter(|pull_request| seen.insert(pull_request.id))
            .cloned()
            .collect();
        merged.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        merged
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

    /// The Review tab merges both lists newest first, a pull request in both once.
    #[test]
    fn review_tab_merges_requested_and_reviewed() {
        let lists = QueueLists {
            review_requested: vec![pull_request(1, "2026-11-03"), pull_request(2, "2026-11-01")],
            reviewed: vec![pull_request(3, "2026-11-02"), pull_request(1, "2026-11-03")],
            ..QueueLists::default()
        };
        let ids: Vec<u64> = lists.review_tab().iter().map(|listed| listed.id).collect();
        assert_eq!(ids, [1, 3, 2]);
    }
}
