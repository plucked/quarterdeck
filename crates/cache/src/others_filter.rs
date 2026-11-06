use qd_file_tree::FacetFilter;
use qd_github::RepoRef;

use crate::UpdatedWithin;

/// How the Others tab narrows one repository's pull requests. Its default
/// narrows nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OthersFilter {
    pub hide_drafts: bool,
    /// Hide the pull requests that request the viewer's review (the Review
    /// tab lists them).
    pub hide_review_requests: bool,
    pub updated_within: UpdatedWithin,
    /// Picked and hidden label names.
    pub labels: FacetFilter<String>,
    /// Picked and hidden author logins.
    pub authors: FacetFilter<String>,
}

impl OthersFilter {
    /// Whether it narrows anything.
    pub fn is_active(&self) -> bool {
        self.hide_drafts
            || self.hide_review_requests
            || self.updated_within != UpdatedWithin::AnyTime
            || self.labels.is_active()
            || self.authors.is_active()
    }
}

/// A repository's [`OthersFilter`], as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryOthersFilter {
    pub repo: RepoRef,
    pub filter: OthersFilter,
}
