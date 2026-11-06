use std::collections::{HashMap, HashSet};

use qd_github::{PullRequest, PullRequestStack, RepoRef};

use super::collapsed_groups::CollapsedGroups;
use super::stack_key::StackKey;

/// One row of the queue's sidebar list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueRow {
    /// The header above a repository's pull requests.
    RepoHeader(RepoGroup),
    /// A standalone pull request, by its index in the shown list.
    PullRequest(usize),
    /// The header above a stack's layers.
    StackHeader(StackGroup),
    /// A layer of the stack above that the shown list has, by its index there.
    StackLayer(usize),
    /// A layer of the stack above that the shown list lacks (someone
    /// else's, or one not asking for the viewer's review), by its index in
    /// the stack's layers.
    OtherLayer { key: StackKey, layer: usize },
    /// The Others tab's filter of the repository above, open under its header.
    OthersFilter(RepoRef),
    /// Under a repository of the Others tab that shows no pull request: it
    /// has none open, or its filter hides them all.
    NothingShown(RepoRef),
}

/// What a repository's header shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoGroup {
    pub repo: RepoRef,
    /// Pull requests of it in the shown list.
    pub pull_requests: usize,
    pub is_collapsed: bool,
    /// Its filter, in the Others tab; `None` in the other tabs.
    pub others: Option<OthersGroup>,
}

/// A repository of the Others tab: how many of its open pull requests
/// there are before the filter, and the filter's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OthersGroup {
    pub total: usize,
    pub is_filter_active: bool,
    pub is_filter_open: bool,
}

/// A repository the Others tab shows, in order, with its [`OthersGroup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OthersRepository {
    pub repo: RepoRef,
    pub group: OthersGroup,
}

/// What a stack's header shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackGroup {
    pub key: StackKey,
    /// Layers in the whole stack.
    pub size: u64,
    pub base_ref: String,
    /// Layers of it in the shown list.
    pub listed: usize,
    pub is_collapsed: bool,
}

impl QueueRow {
    /// The rows for `list`, grouped by repository: each repository's header
    /// (in the order of its most recently updated pull request, as the list
    /// is sorted), then, unless it is collapsed, its pull requests in list
    /// order. A stack's header takes the place of its first listed layer,
    /// and its layers follow bottom first unless it is collapsed. With the
    /// stack's layers in `stacks`, every layer shows, listed or not;
    /// without, only the listed ones.
    pub fn build_all(
        list: &[PullRequest],
        collapsed: &CollapsedGroups,
        stacks: &HashMap<StackKey, PullRequestStack>,
    ) -> Vec<Self> {
        // The repositories in the order they first appear.
        let mut repositories: Vec<&RepoRef> = Vec::new();
        for pull_request in list {
            if !repositories.contains(&&pull_request.repo) {
                repositories.push(&pull_request.repo);
            }
        }

        // Each one's header, then its rows.
        let mut rows = Vec::new();
        for repo in repositories {
            let indexes: Vec<usize> = list
                .iter()
                .enumerate()
                .filter(|(_, pull_request)| pull_request.repo == *repo)
                .map(|(index, _)| index)
                .collect();
            let is_collapsed = collapsed.repositories.contains(repo);
            rows.push(Self::RepoHeader(RepoGroup {
                repo: repo.clone(),
                pull_requests: indexes.len(),
                is_collapsed,
                others: None,
            }));
            if !is_collapsed {
                rows.extend(repository_rows(list, &indexes, &collapsed.stacks, stacks));
            }
        }
        rows
    }

    /// The rows of the Others tab: every repository of `repositories`, in
    /// that order, even when it shows nothing (so its filter stays within
    /// reach): its header, its filter when open, then its pull requests in
    /// `shown` (already filtered) grouped like [`Self::build_all`], or a
    /// note that none shows. A collapsed repository shows its header alone.
    pub fn build_others(
        shown: &[PullRequest],
        repositories: &[OthersRepository],
        collapsed: &CollapsedGroups,
        stacks: &HashMap<StackKey, PullRequestStack>,
    ) -> Vec<Self> {
        let mut rows = Vec::new();
        for repository in repositories {
            let repo = &repository.repo;
            let indexes: Vec<usize> = shown
                .iter()
                .enumerate()
                .filter(|(_, pull_request)| &pull_request.repo == repo)
                .map(|(index, _)| index)
                .collect();
            let is_collapsed = collapsed.repositories.contains(repo);
            rows.push(Self::RepoHeader(RepoGroup {
                repo: repo.clone(),
                pull_requests: indexes.len(),
                is_collapsed,
                others: Some(repository.group.clone()),
            }));
            if is_collapsed {
                continue;
            }
            if repository.group.is_filter_open {
                rows.push(Self::OthersFilter(repo.clone()));
            }
            if indexes.is_empty() {
                rows.push(Self::NothingShown(repo.clone()));
            } else {
                rows.extend(repository_rows(shown, &indexes, &collapsed.stacks, stacks));
            }
        }
        rows
    }

    /// The index in the shown list of the pull request this row shows.
    pub fn pull_request_index(&self) -> Option<usize> {
        match self {
            Self::PullRequest(index) | Self::StackLayer(index) => Some(*index),
            Self::RepoHeader(_)
            | Self::StackHeader(_)
            | Self::OtherLayer { .. }
            | Self::OthersFilter(_)
            | Self::NothingShown(_) => None,
        }
    }
}

/// The rows of one repository's pull requests, at `indexes` in `list`: a
/// standalone pull request is one row; a stack is placed once, at its first
/// listed layer, as its header and (unless collapsed) its layers.
fn repository_rows(
    list: &[PullRequest],
    indexes: &[usize],
    collapsed_stacks: &HashSet<StackKey>,
    stacks: &HashMap<StackKey, PullRequestStack>,
) -> Vec<QueueRow> {
    let mut rows = Vec::new();
    let mut placed: HashSet<StackKey> = HashSet::new();
    for &index in indexes {
        let pull_request = &list[index];
        let (Some(key), Some(membership)) = (StackKey::of(pull_request), &pull_request.stack)
        else {
            rows.push(QueueRow::PullRequest(index));
            continue;
        };
        if !placed.insert(key.clone()) {
            continue;
        }

        // Its listed layers, bottom first.
        let mut listed: Vec<usize> = list
            .iter()
            .enumerate()
            .filter(|(_, candidate)| StackKey::of(candidate).as_ref() == Some(&key))
            .map(|(layer_index, _)| layer_index)
            .collect();
        listed.sort_by_key(|layer_index| {
            list[*layer_index]
                .stack
                .as_ref()
                .map_or(0, |membership| membership.position)
        });

        // The header, then every layer when the stack is known, else the listed ones.
        let stack = stacks.get(&key);
        let layers = match stack {
            Some(stack) => stack_layer_rows(&key, stack, list, &listed),
            None => listed.iter().copied().map(QueueRow::StackLayer).collect(),
        };
        let is_collapsed = collapsed_stacks.contains(&key);
        rows.push(QueueRow::StackHeader(StackGroup {
            size: stack.map_or(membership.size, |stack| {
                u64::try_from(stack.layers.len()).unwrap_or(u64::MAX)
            }),
            base_ref: stack.map_or_else(
                || membership.base_ref.clone(),
                |stack| stack.base_ref.clone(),
            ),
            listed: listed.len(),
            is_collapsed,
            key,
        }));
        if !is_collapsed {
            rows.extend(layers);
        }
    }
    rows
}

/// Every layer of `stack` bottom first: listed ones by their list index,
/// the others by their place in the stack; listed layers the stack no
/// longer has (it changed since) follow at the end.
fn stack_layer_rows(
    key: &StackKey,
    stack: &PullRequestStack,
    list: &[PullRequest],
    listed: &[usize],
) -> Vec<QueueRow> {
    let mut rows: Vec<QueueRow> = stack
        .layers
        .iter()
        .enumerate()
        .map(|(layer_index, layer)| {
            listed
                .iter()
                .find(|index| list[**index].number == layer.number)
                .map_or_else(
                    || QueueRow::OtherLayer {
                        key: key.clone(),
                        layer: layer_index,
                    },
                    |index| QueueRow::StackLayer(*index),
                )
        })
        .collect();
    rows.extend(
        listed
            .iter()
            .filter(|index| {
                !stack
                    .layers
                    .iter()
                    .any(|layer| layer.number == list[**index].number)
            })
            .map(|index| QueueRow::StackLayer(*index)),
    );
    rows
}

#[cfg(test)]
mod tests {
    use qd_github::{RepoRef, StackLayer, StackMembership, User};

    use super::*;

    fn pull_request(number: u64, stack: Option<(u64, u64)>) -> PullRequest {
        in_repo("b", number, stack)
    }

    fn in_repo(name: &str, number: u64, stack: Option<(u64, u64)>) -> PullRequest {
        PullRequest {
            id: number,
            number,
            repo: RepoRef {
                owner: "a".into(),
                name: name.into(),
            },
            title: String::new(),
            body: None,
            author: User {
                login: String::new(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft: false,
            created_at: String::new(),
            updated_at: String::new(),
            labels: Vec::new(),
            stack: stack.map(|(stack_number, position)| StackMembership {
                number: stack_number,
                position,
                size: 3,
                base_ref: "main".into(),
            }),
        }
    }

    /// A stack is grouped where its first layer appears, its layers bottom first.
    #[test]
    fn groups_stack_layers_bottom_first() {
        let list = [
            pull_request(10, Some((9, 3))),
            pull_request(11, None),
            pull_request(12, Some((9, 1))),
        ];
        let rows = QueueRow::build_all(&list, &CollapsedGroups::default(), &HashMap::new());
        assert!(matches!(&rows[0], QueueRow::RepoHeader(group) if group.pull_requests == 3));
        assert!(
            matches!(&rows[1], QueueRow::StackHeader(group) if group.listed == 2 && group.size == 3)
        );
        assert_eq!(
            rows[2..],
            [
                QueueRow::StackLayer(2),
                QueueRow::StackLayer(0),
                QueueRow::PullRequest(1)
            ]
        );
    }

    /// A collapsed stack shows its header alone.
    #[test]
    fn collapsed_stack_shows_header_only() {
        let list = [
            pull_request(10, Some((9, 1))),
            pull_request(11, Some((9, 2))),
        ];
        let collapsed = CollapsedGroups {
            stacks: HashSet::from([StackKey::of(&list[0]).unwrap()]),
            ..CollapsedGroups::default()
        };
        let rows = QueueRow::build_all(&list, &collapsed, &HashMap::new());
        assert_eq!(rows.len(), 2);
        assert!(matches!(&rows[1], QueueRow::StackHeader(group) if group.is_collapsed));
    }

    /// Pull requests group under their repository, repositories in the order
    /// of their most recent pull request; a collapsed one shows its header alone.
    #[test]
    fn groups_pull_requests_by_repository() {
        let list = [
            in_repo("web", 1, None),
            in_repo("api", 2, None),
            in_repo("web", 3, None),
        ];
        let rows = QueueRow::build_all(&list, &CollapsedGroups::default(), &HashMap::new());
        let header = |name: &str, count: usize| {
            QueueRow::RepoHeader(RepoGroup {
                repo: list
                    .iter()
                    .find(|pr| pr.repo.name == name)
                    .unwrap()
                    .repo
                    .clone(),
                pull_requests: count,
                is_collapsed: false,
                others: None,
            })
        };
        assert_eq!(
            rows,
            [
                header("web", 2),
                QueueRow::PullRequest(0),
                QueueRow::PullRequest(2),
                header("api", 1),
                QueueRow::PullRequest(1),
            ]
        );

        let collapsed = CollapsedGroups {
            repositories: HashSet::from([list[0].repo.clone()]),
            ..CollapsedGroups::default()
        };
        let rows = QueueRow::build_all(&list, &collapsed, &HashMap::new());
        assert_eq!(rows.len(), 3);
        assert!(matches!(&rows[0], QueueRow::RepoHeader(group) if group.is_collapsed));
    }

    fn layer(number: u64) -> StackLayer {
        StackLayer {
            pull_request_id: number,
            number,
            title: String::new(),
            is_open: true,
            is_draft: false,
            is_merged: false,
            html_url: String::new(),
            head_ref: String::new(),
            head_sha: String::new(),
            base_ref: String::new(),
            base_sha: String::new(),
            author_login: String::new(),
        }
    }

    /// The Others tab lists every repository, an empty one with a note, and
    /// an open filter right under its header.
    #[test]
    fn others_rows_keep_empty_repositories() {
        let shown = [in_repo("web", 1, None)];
        let repository = |name: &str, is_filter_open: bool| OthersRepository {
            repo: RepoRef {
                owner: "a".into(),
                name: name.into(),
            },
            group: OthersGroup {
                total: 3,
                is_filter_active: true,
                is_filter_open,
            },
        };
        let repositories = [repository("web", true), repository("api", false)];
        let rows = QueueRow::build_others(
            &shown,
            &repositories,
            &CollapsedGroups::default(),
            &HashMap::new(),
        );
        assert_eq!(rows.len(), 5);
        assert!(matches!(&rows[0], QueueRow::RepoHeader(group) if group.pull_requests == 1));
        assert_eq!(
            rows[1],
            QueueRow::OthersFilter(repositories[0].repo.clone())
        );
        assert_eq!(rows[2], QueueRow::PullRequest(0));
        assert!(matches!(&rows[3], QueueRow::RepoHeader(group) if group.pull_requests == 0));
        assert_eq!(
            rows[4],
            QueueRow::NothingShown(repositories[1].repo.clone())
        );
    }

    /// With the stack known, every layer shows: listed ones by list index, others by stack place.
    #[test]
    fn known_stack_shows_every_layer() {
        let list = [pull_request(12, Some((9, 2)))];
        let key = StackKey::of(&list[0]).unwrap();
        let stack = PullRequestStack {
            repo: key.repo.clone(),
            number: 9,
            base_ref: "main".into(),
            is_open: true,
            layers: vec![layer(11), layer(12), layer(13)],
        };
        let rows = QueueRow::build_all(
            &list,
            &CollapsedGroups::default(),
            &HashMap::from([(key.clone(), stack)]),
        );
        assert!(
            matches!(&rows[1], QueueRow::StackHeader(group) if group.listed == 1 && group.size == 3)
        );
        assert_eq!(
            rows[2..],
            [
                QueueRow::OtherLayer {
                    key: key.clone(),
                    layer: 0
                },
                QueueRow::StackLayer(0),
                QueueRow::OtherLayer { key, layer: 2 },
            ]
        );
    }
}
