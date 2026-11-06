use std::collections::{BTreeMap, HashSet};

use qd_cache::OthersFilter;
use qd_file_tree::FacetCount;
use qd_github::PullRequest;

use crate::display_date::unix_seconds;

/// A chip row of the Others filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OthersFacet {
    Labels,
    Authors,
}

/// What an Others filter decides from besides the filter itself: which
/// pull requests ask for the viewer's review, and the time now.
pub struct FilterContext<'a> {
    /// Ids of the pull requests in the Review tab.
    pub review_requested: &'a HashSet<u64>,
    /// Unix seconds.
    pub now: i64,
}

/// Whether `pull_request` passes `filter`.
pub fn passes(filter: &OthersFilter, pull_request: &PullRequest, context: &FilterContext) -> bool {
    passes_switches(filter, pull_request, context)
        && filter.labels.admits_any(label_names(pull_request))
        && filter.authors.admits(pull_request.author.login.as_str())
}

/// The chips of one row for `pull_requests` (one repository's), those the
/// rest of the filter lets through most first (so chips that would show
/// nothing sink to the end), then the most common, then by name: how many
/// have each value, and how many of those the rest of the filter lets through.
pub fn chip_counts(
    filter: &OthersFilter,
    facet: OthersFacet,
    pull_requests: &[PullRequest],
    context: &FilterContext,
) -> Vec<FacetCount<String>> {
    // Count each value, and the pull requests with it that the other rows admit.
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for pull_request in pull_requests {
        let passes_rest = passes_switches(filter, pull_request, context)
            && match facet {
                OthersFacet::Labels => filter.authors.admits(pull_request.author.login.as_str()),
                OthersFacet::Authors => filter.labels.admits_any(label_names(pull_request)),
            };
        let values: Vec<&str> = match facet {
            OthersFacet::Labels => pull_request
                .labels
                .iter()
                .map(|label| label.name.as_str())
                .collect(),
            OthersFacet::Authors => vec![pull_request.author.login.as_str()],
        };
        for value in values {
            let entry = counts.entry(value).or_default();
            entry.0 += 1;
            entry.1 += usize::from(passes_rest);
        }
    }

    // Most matching first, so the collapsed row shows the useful ones.
    let mut counts: Vec<FacetCount<String>> = counts
        .into_iter()
        .map(|(value, (total, matching))| FacetCount {
            value: value.to_owned(),
            total,
            matching,
        })
        .collect();
    counts.sort_by(|left, right| {
        right
            .matching
            .cmp(&left.matching)
            .then_with(|| right.total.cmp(&left.total))
            .then_with(|| left.value.to_lowercase().cmp(&right.value.to_lowercase()))
    });
    counts
}

/// What narrows a repository's pull requests, in words for its empty note:
/// `bug`, `not dependabot[bot]`, `no drafts`.
pub fn describe(filter: &OthersFilter) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    parts.extend(filter.labels.only.iter().cloned());
    parts.extend(filter.authors.only.iter().cloned());
    parts.extend(
        filter
            .labels
            .hidden
            .iter()
            .map(|value| format!("not {value}")),
    );
    parts.extend(
        filter
            .authors
            .hidden
            .iter()
            .map(|value| format!("not {value}")),
    );
    if filter.hide_drafts {
        parts.push("no drafts".to_owned());
    }
    if filter.hide_review_requests {
        parts.push("no review requests".to_owned());
    }
    if let Some(seconds) = filter.updated_within.seconds() {
        parts.push(format!("updated in {} days", seconds / (24 * 60 * 60)));
    }
    parts
}

/// The switches: drafts, review requests and how recently it was updated.
fn passes_switches(
    filter: &OthersFilter,
    pull_request: &PullRequest,
    context: &FilterContext,
) -> bool {
    let is_recent = filter.updated_within.seconds().is_none_or(|limit| {
        // An unreadable time is kept rather than hidden.
        unix_seconds(&pull_request.updated_at).is_none_or(|updated| context.now - updated <= limit)
    });
    let is_hidden_draft = filter.hide_drafts && pull_request.draft;
    let is_hidden_request =
        filter.hide_review_requests && context.review_requested.contains(&pull_request.id);
    !is_hidden_draft && !is_hidden_request && is_recent
}

/// The names of `pull_request`'s labels.
fn label_names(pull_request: &PullRequest) -> impl Iterator<Item = &String> + Clone {
    pull_request.labels.iter().map(|label| &label.name)
}

#[cfg(test)]
mod tests {
    use qd_cache::UpdatedWithin;
    use qd_github::{Label, RepoRef, User};

    use super::*;

    /// 2026-11-06T12:00:00Z.
    const NOW: i64 = 1_793_966_400;

    fn pull_request(
        id: u64,
        author: &str,
        labels: &[&str],
        draft: bool,
        updated_at: &str,
    ) -> PullRequest {
        PullRequest {
            id,
            number: id,
            repo: RepoRef {
                owner: "o".into(),
                name: "web".into(),
            },
            title: String::new(),
            body: None,
            author: User {
                login: author.into(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft,
            created_at: String::new(),
            updated_at: updated_at.into(),
            labels: labels
                .iter()
                .map(|name| Label {
                    name: (*name).into(),
                    color: "ffffff".into(),
                })
                .collect(),
            stack: None,
        }
    }

    fn pull_requests() -> Vec<PullRequest> {
        vec![
            pull_request(1, "ana", &["bug", "ui"], false, "2026-11-05T12:00:00Z"),
            pull_request(2, "bo", &["bug"], true, "2026-11-01T12:00:00Z"),
            pull_request(
                3,
                "dependabot[bot]",
                &["deps"],
                false,
                "2026-09-01T12:00:00Z",
            ),
            pull_request(4, "ana", &[], false, "2026-11-06T11:00:00Z"),
        ]
    }

    fn shown(filter: &OthersFilter, review_requested: &HashSet<u64>) -> Vec<u64> {
        let context = FilterContext {
            review_requested,
            now: NOW,
        };
        pull_requests()
            .iter()
            .filter(|pull_request| passes(filter, pull_request, &context))
            .map(|pull_request| pull_request.id)
            .collect()
    }

    /// The default filter shows every pull request.
    #[test]
    fn shows_all_by_default() {
        assert_eq!(
            shown(&OthersFilter::default(), &HashSet::new()),
            [1, 2, 3, 4]
        );
    }

    /// Labels pick any of their values; hidden authors go; switches narrow.
    #[test]
    fn narrows_by_chips_and_switches() {
        let mut filter = OthersFilter::default();
        filter.labels.toggle_only(&"bug".to_owned());
        assert_eq!(shown(&filter, &HashSet::new()), [1, 2]);

        filter.hide_drafts = true;
        assert_eq!(shown(&filter, &HashSet::new()), [1]);

        let mut filter = OthersFilter::default();
        filter.authors.toggle_hidden(&"dependabot[bot]".to_owned());
        filter.hide_review_requests = true;
        assert_eq!(shown(&filter, &HashSet::from([4])), [1, 2]);

        let filter = OthersFilter {
            updated_within: UpdatedWithin::Week,
            ..OthersFilter::default()
        };
        assert_eq!(shown(&filter, &HashSet::new()), [1, 2, 4]);
    }

    /// Chip counts: most matching first, and what the other row lets through.
    #[test]
    fn counts_chips_given_the_other_row() {
        let mut filter = OthersFilter::default();
        filter.authors.toggle_only(&"ana".to_owned());
        let context = FilterContext {
            review_requested: &HashSet::new(),
            now: NOW,
        };
        let labels = chip_counts(&filter, OthersFacet::Labels, &pull_requests(), &context);
        let summary: Vec<(&str, usize, usize)> = labels
            .iter()
            .map(|count| (count.value.as_str(), count.total, count.matching))
            .collect();
        assert_eq!(summary, [("bug", 2, 1), ("ui", 1, 1), ("deps", 1, 0)]);
    }
}
