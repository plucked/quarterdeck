use std::collections::HashMap;
use std::time::Duration;

use qd_cache::PullRequestListKey;
use qd_github::{
    GitHubClient, PullRequest, PullRequestBranches, RepoRef, StackMembership, ViewerReview,
};

use super::queue_lists::QueueLists;
use crate::shared_cache::SharedCache;

/// What GraphQL tells of a listed pull request that the search and issues
/// APIs do not: its branches, the viewer's latest review and its stack membership.
#[derive(Debug, Clone, Default)]
pub struct ListedDetails {
    pub branches: Option<PullRequestBranches>,
    pub viewer_review: Option<ViewerReview>,
    pub stack: Option<StackMembership>,
}

/// Listed details by repository and pull request number.
pub type DetailsByNumber = HashMap<(RepoRef, u64), ListedDetails>;

/// The viewer's own lists as a refresh fetched them (the first part of a
/// refresh), with their branches and stacks filled in, and the details for others'.
pub struct OwnLists {
    pub authored: Vec<PullRequest>,
    pub review_requested: Vec<PullRequest>,
    pub reviewed: Vec<PullRequest>,
    pub details: DetailsByNumber,
    /// Why the details could not be read, when they could not (the lists keep the last known ones).
    pub details_error: Option<String>,
}

/// Others' pull requests as a refresh fetched them (its second part).
pub struct FetchedOthers {
    pub pull_requests: Vec<PullRequest>,
    /// Where each repository's next sync starts.
    pub synced_until: HashMap<RepoRef, String>,
    /// Why others' pull requests could not be read, when they could not
    /// (the repositories that failed keep the last known ones).
    pub error: Option<String>,
    /// How long to wait before asking GitHub again (a used-up budget or a
    /// `retry-after`), when known.
    pub retry_after: Option<Duration>,
}

/// What a refresh starts from besides the client: the watched repositories
/// and what is known already, kept when part of the refresh fails.
pub struct RefreshInput {
    pub watched: Vec<RepoRef>,
    pub known_details: DetailsByNumber,
    pub known_others: Vec<PullRequest>,
    /// Where each repository's last sync this session ended.
    pub others_synced_until: HashMap<RepoRef, String>,
}

/// The cached lists, read at startup.
pub fn read_cached_lists(cache: &SharedCache) -> anyhow::Result<QueueLists> {
    let cache = cache.lock();
    Ok(QueueLists {
        authored: cache.list(PullRequestListKey::AUTHORED)?,
        review_requested: cache.list(PullRequestListKey::REVIEW_REQUESTED)?,
        reviewed: cache.list(PullRequestListKey::REVIEWED)?,
        others: cache.list(PullRequestListKey::OTHERS)?,
    })
}

/// The branches and stack memberships the lists know now, kept when GraphQL fails.
pub fn known_details(lists: &QueueLists) -> DetailsByNumber {
    lists
        .iter()
        .map(|pull_request| {
            let details = ListedDetails {
                branches: pull_request.branches.clone(),
                viewer_review: pull_request.viewer_review,
                stack: pull_request.stack.clone(),
            };
            ((pull_request.repo.clone(), pull_request.number), details)
        })
        .collect()
}

/// Fetches the viewer's own lists, those requesting the viewer's review and
/// those the viewer reviewed (one search request each), and the branches,
/// viewer reviews and stacks of every list (one GraphQL request, others' in
/// `watched` included), fills them in and caches the lists. When GraphQL
/// cannot be read, the details known before are kept.
///
/// Blocking: runs on the background executor.
pub fn fetch_own_lists(
    client: &GitHubClient,
    cache: &SharedCache,
    watched: &[RepoRef],
    known_details: DetailsByNumber,
) -> anyhow::Result<OwnLists> {
    // The three lists, then the branches and stacks of all of them in one request.
    let mut authored = client.authored()?;
    let mut review_requested = client.review_requested()?;
    let mut reviewed = client.reviewed_by_viewer()?;
    let (details, details_error) = match client.listed_details(watched) {
        Ok(listed) => (
            listed
                .into_iter()
                .map(|listed| {
                    let details = ListedDetails {
                        branches: Some(listed.branches),
                        viewer_review: listed.viewer_review,
                        stack: listed.stack,
                    };
                    ((listed.repo, listed.number), details)
                })
                .collect(),
            None,
        ),
        Err(error) => (known_details, Some(format!("{error:#}"))),
    };
    fill_details(&mut authored, &details);
    fill_details(&mut review_requested, &details);
    fill_details(&mut reviewed, &details);

    // Cache them before showing them.
    let mut cache = cache.lock();
    cache.replace_list(PullRequestListKey::AUTHORED, &authored)?;
    cache.replace_list(PullRequestListKey::REVIEW_REQUESTED, &review_requested)?;
    cache.replace_list(PullRequestListKey::REVIEWED, &reviewed)?;
    Ok(OwnLists {
        authored,
        review_requested,
        reviewed,
        details,
        details_error,
    })
}

/// Sets each of `pull_requests`' branches, viewer review and stack
/// membership from `details` (no membership for a standalone one).
fn fill_details(pull_requests: &mut [PullRequest], details: &DetailsByNumber) {
    for pull_request in pull_requests {
        let found = details
            .get(&(pull_request.repo.clone(), pull_request.number))
            .cloned()
            .unwrap_or_default();
        pull_request.branches = found.branches;
        pull_request.viewer_review = found.viewer_review;
        pull_request.stack = found.stack;
    }
}

/// Sets each of others' `pull_requests`' stack membership from `details`
/// (none for a standalone one). Their branches and reviews stay: GraphQL's
/// others' search covers only the newest, and their sync looks the rest up.
pub fn fill_stacks(pull_requests: &mut [PullRequest], details: &DetailsByNumber) {
    for pull_request in pull_requests {
        pull_request.stack = details
            .get(&(pull_request.repo.clone(), pull_request.number))
            .and_then(|found| found.stack.clone());
    }
}

/// Others' pull requests brought up to date ([`fetch_others`]) with the
/// stack memberships in `details`, cached as the Others list; then cached
/// data unused within `ttl` is removed.
///
/// Blocking: runs on the background executor.
pub fn fetch_and_cache_others(
    client: &GitHubClient,
    cache: &SharedCache,
    input: &RefreshInput,
    details: &DetailsByNumber,
    ttl: Duration,
) -> anyhow::Result<FetchedOthers> {
    // Others' pull requests, with their stacks filled in.
    let mut fetched = fetch_others(
        client,
        &input.watched,
        &input.known_others,
        &input.others_synced_until,
    );
    fill_stacks(&mut fetched.pull_requests, details);

    // Cache them, then drop what has not been used within the TTL.
    let mut cache = cache.lock();
    cache.replace_list(PullRequestListKey::OTHERS, &fetched.pull_requests)?;
    cache.delete_expired(ttl)?;
    Ok(fetched)
}

/// Others' open pull requests in `watched`, each repository brought up to
/// date from what `known` has of it and where its last sync ended
/// (`synced_until`), most recently updated first. A repository that fails
/// keeps what was known of it, and the first failure is returned to show;
/// once GitHub refuses requests (rate limit, slow down), the rest are not
/// asked and keep what was known too.
///
/// Blocking: runs on the background executor.
fn fetch_others(
    client: &GitHubClient,
    watched: &[RepoRef],
    known: &[PullRequest],
    synced_until: &HashMap<RepoRef, String>,
) -> FetchedOthers {
    let known_of = |repo: &RepoRef| -> Vec<PullRequest> {
        known
            .iter()
            .filter(|pull_request| &pull_request.repo == repo)
            .cloned()
            .collect()
    };
    let mut fetched = FetchedOthers {
        pull_requests: Vec::new(),
        synced_until: HashMap::new(),
        error: None,
        retry_after: None,
    };
    if watched.is_empty() {
        return fetched;
    }

    // Whose pull requests to leave out: the viewer's (asked once per session).
    let viewer_login = match client.viewer_login() {
        Ok(login) => login,
        Err(error) => {
            fetched.pull_requests = known.to_vec();
            fetched.synced_until.clone_from(synced_until);
            fetched.retry_after = error.retry_after();
            fetched.error = Some(format!("{error:#}"));
            return fetched;
        }
    };

    // Each repository on its own, so one failure keeps the others current.
    let mut is_throttled = false;
    for repo in watched {
        let known = known_of(repo);
        let last_synced = synced_until.get(repo);
        let result = (!is_throttled).then(|| {
            client.other_pull_requests(repo, &known, last_synced.map(String::as_str), &viewer_login)
        });
        match result {
            Some(Ok(update)) => {
                fetched.pull_requests.extend(update.pull_requests);
                if let Some(until) = update.synced_until {
                    fetched.synced_until.insert(repo.clone(), until);
                }
            }
            Some(Err(error)) => {
                is_throttled = error.is_throttled();
                fetched.retry_after = fetched.retry_after.or(error.retry_after());
                fetched
                    .error
                    .get_or_insert_with(|| format!("{}: {error:#}", repo.full_name()));
                keep_known(&mut fetched, repo, known, last_synced);
            }
            None => keep_known(&mut fetched, repo, known, last_synced),
        }
    }
    fetched
        .pull_requests
        .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    fetched
}

/// Keeps what was known of `repo` (its pull requests and where its last
/// sync ended) in `fetched`, when it could not be brought up to date.
fn keep_known(
    fetched: &mut FetchedOthers,
    repo: &RepoRef,
    known: Vec<PullRequest>,
    last_synced: Option<&String>,
) {
    fetched.pull_requests.extend(known);
    if let Some(until) = last_synced {
        fetched.synced_until.insert(repo.clone(), until.clone());
    }
}
