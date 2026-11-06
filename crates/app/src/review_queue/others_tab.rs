use std::collections::{HashMap, HashSet};

use gpui::SharedString;
use qd_cache::{OthersFilter, RepositoryOthersFilter};
use qd_github::{PullRequest, RepoRef};

use super::chip_marks::ChipMarks;
use super::open_filter_chips::OpenFilterChips;
use super::others_filtering::{FilterContext, OthersFacet, chip_counts, passes};
use super::queue_lists::QueueLists;
use super::queue_row::{OthersGroup, OthersRepository};

/// The Others tab's state: the watched repositories, each one's filter,
/// which pull requests pass, and what the sidebar has open.
#[derive(Default)]
pub struct OthersTab {
    /// The repositories the tab lists, as last seen in the settings.
    watched: Vec<RepoRef>,
    /// Each repository's filter; a repository without one shows all.
    filters: HashMap<RepoRef, OthersFilter>,
    /// Others' pull requests that pass their repository's filter, in list
    /// order: what the tab's rows index into.
    shown: Vec<PullRequest>,
    /// The repository whose filter is open under its header.
    open_filter: Option<RepoRef>,
    /// The chips of that filter, worked out with what passes.
    open_filter_chips: Option<OpenFilterChips>,
    /// Ids of the pull requests in the Review tab, for Hide review requests.
    review_requested: HashSet<u64>,
    /// The chip rows showing all their chips, not just the most common.
    expanded_chip_rows: HashSet<(RepoRef, OthersFacet)>,
    /// Why others' pull requests could not be refreshed, until they can.
    error: Option<SharedString>,
    /// Where each repository's last sync ended this session (GitHub's
    /// `updated_at` of the newest item read), so the next asks only for
    /// changes since.
    synced_until: HashMap<RepoRef, String>,
}

impl OthersTab {
    /// The tab for `watched` repositories with the filters saved before.
    pub fn new(watched: Vec<RepoRef>, saved_filters: Vec<RepositoryOthersFilter>) -> Self {
        Self {
            watched,
            filters: saved_filters
                .into_iter()
                .map(|saved| (saved.repo, saved.filter))
                .collect(),
            ..Self::default()
        }
    }

    pub fn watched(&self) -> &[RepoRef] {
        &self.watched
    }

    /// Whether any repository is watched (the tab has something to list).
    pub fn has_watched(&self) -> bool {
        !self.watched.is_empty()
    }

    /// Takes the watched repositories from the settings; whether they changed.
    pub fn set_watched(&mut self, watched: &[RepoRef]) -> bool {
        if watched == self.watched.as_slice() {
            return false;
        }
        self.watched = watched.to_vec();
        true
    }

    /// The pull requests that pass their repository's filter.
    pub fn shown(&self) -> &[PullRequest] {
        &self.shown
    }

    /// `repo`'s filter; the default (narrowing nothing) when it has none.
    pub fn filter(&self, repo: &RepoRef) -> OthersFilter {
        self.filters.get(repo).cloned().unwrap_or_default()
    }

    /// Changes `repo`'s filter with `change` and returns it, to be recorded.
    pub fn change_filter(
        &mut self,
        repo: &RepoRef,
        change: impl FnOnce(&mut OthersFilter),
    ) -> OthersFilter {
        let filter = self.filters.entry(repo.clone()).or_default();
        change(filter);
        filter.clone()
    }

    pub fn synced_until(&self) -> &HashMap<RepoRef, String> {
        &self.synced_until
    }

    pub fn set_synced_until(&mut self, synced_until: HashMap<RepoRef, String>) {
        self.synced_until = synced_until;
    }

    pub fn error(&self) -> Option<&SharedString> {
        self.error.as_ref()
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.error = error.map(SharedString::from);
    }

    /// The chips of the open filter, once worked out by [`Self::update_shown`].
    pub fn open_filter_chips(&self) -> Option<&OpenFilterChips> {
        self.open_filter_chips.as_ref()
    }

    /// Opens `repo`'s filter (closing another one), or closes it; its chips
    /// are worked out by the next [`Self::update_shown`].
    pub fn toggle_filter_panel(&mut self, repo: &RepoRef) {
        self.open_filter = match &self.open_filter {
            Some(open) if open == repo => None,
            _ => Some(repo.clone()),
        };
    }

    /// Whether `facet`'s chip row of `repo` shows all its chips.
    pub fn is_chip_row_expanded(&self, repo: &RepoRef, facet: OthersFacet) -> bool {
        self.expanded_chip_rows.contains(&(repo.clone(), facet))
    }

    /// Shows all of a chip row's chips, or only the most common again.
    pub fn toggle_chip_row(&mut self, repo: &RepoRef, facet: OthersFacet) {
        let key = (repo.clone(), facet);
        if !self.expanded_chip_rows.remove(&key) {
            self.expanded_chip_rows.insert(key);
        }
    }

    /// Works out which of `lists.others` pass their repository's filter
    /// (only watched repositories count), and the chips of the open filter.
    pub fn update_shown(&mut self, lists: &QueueLists, now: i64) {
        self.review_requested = review_requested_ids(lists);
        let context = FilterContext {
            review_requested: &self.review_requested,
            now,
        };
        self.shown = lists
            .others
            .iter()
            .filter(|pull_request| {
                self.watched.contains(&pull_request.repo)
                    && self
                        .filters
                        .get(&pull_request.repo)
                        .is_none_or(|filter| passes(filter, pull_request, &context))
            })
            .cloned()
            .collect();

        // The open filter's chips, counted over its repository's pull requests.
        self.open_filter_chips = self.open_filter.as_ref().map(|repo| {
            let filter = self.filters.get(repo).cloned().unwrap_or_default();
            let pull_requests: Vec<&PullRequest> = lists
                .others
                .iter()
                .filter(|pull_request| &pull_request.repo == repo)
                .collect();
            OpenFilterChips {
                labels: chip_counts(&filter, OthersFacet::Labels, &pull_requests, &context),
                authors: chip_counts(&filter, OthersFacet::Authors, &pull_requests, &context),
                marks: ChipMarks::of(&pull_requests),
            }
        });
    }

    /// The repositories in the order they show: those with open pull
    /// requests by their most recently updated one, then the rest in the
    /// order they were added; each with its counts and filter state.
    pub fn repositories(&self, lists: &QueueLists) -> Vec<OthersRepository> {
        // The watched repositories with pull requests, most recent first, then the others.
        let mut order: Vec<&RepoRef> = Vec::new();
        for pull_request in &lists.others {
            if self.watched.contains(&pull_request.repo) && !order.contains(&&pull_request.repo) {
                order.push(&pull_request.repo);
            }
        }
        for repo in &self.watched {
            if !order.contains(&repo) {
                order.push(repo);
            }
        }

        // Each one's counts and filter.
        order
            .into_iter()
            .map(|repo| OthersRepository {
                repo: repo.clone(),
                group: OthersGroup {
                    total: lists
                        .others
                        .iter()
                        .filter(|pull_request| &pull_request.repo == repo)
                        .count(),
                    is_filter_active: self.filters.get(repo).is_some_and(OthersFilter::is_active),
                    is_filter_open: self.open_filter.as_ref() == Some(repo),
                },
            })
            .collect()
    }
}

/// The ids of the pull requests in the Review tab, for the filters' Hide
/// review requests switch.
pub fn review_requested_ids(lists: &QueueLists) -> HashSet<u64> {
    lists
        .review_requested
        .iter()
        .map(|pull_request| pull_request.id)
        .collect()
}
