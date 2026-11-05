use qd_cache::QueueTab;
use qd_github::{PullRequest, RepoRef};

/// The queue's two lists: the viewer's own pull requests and those that
/// request the viewer's review. A pull request can be in both.
#[derive(Debug, Default)]
pub struct QueueLists {
    pub authored: Vec<PullRequest>,
    pub review_requested: Vec<PullRequest>,
}

impl QueueLists {
    /// The list a tab shows.
    pub fn get(&self, tab: QueueTab) -> &[PullRequest] {
        match tab {
            QueueTab::Authored => &self.authored,
            QueueTab::ReviewRequested => &self.review_requested,
        }
    }

    /// Every listed pull request, the authored list first.
    pub fn iter(&self) -> impl Iterator<Item = &PullRequest> {
        self.authored.iter().chain(&self.review_requested)
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
