use std::collections::{HashMap, HashSet};

use super::queue_trees::QueueTrees;
use qd_github::{PullRequest, RepoRef};

use super::collapsed_groups::CollapsedGroups;
use super::others_group::OthersGroup;
use super::others_repository::OthersRepository;
use super::repo_group::RepoGroup;
use super::stack_group::StackGroup;
use super::stack_key::StackKey;
use crate::distinct_repositories::distinct_repositories;

/// One row of the queue's sidebar list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueRow {
    /// The header above a repository's pull requests.
    RepoHeader(RepoGroup),
    /// A standalone pull request, by its index in the shown list.
    PullRequest(usize),
    /// The header above a stack's layers.
    StackHeader(StackGroup),
    /// A node in the displayed tree, and its index in this tab when listed.
    Layer {
        key: StackKey,
        entry: usize,
        listed: Option<usize>,
    },
    /// The Others tab's filter of the repository above, open under its header.
    OthersFilter(RepoRef),
    /// Under a repository of the Others tab that shows no pull request: it
    /// has none open, or its filter hides them all.
    NothingShown(RepoRef),
}

impl QueueRow {
    /// The rows for `list`, grouped by repository: each repository's header
    /// (in the order of its most recently updated pull request, as the list
    /// is sorted), then, unless it is collapsed, its pull requests in list
    /// order. A stack's header takes the place of its first listed layer,
    /// and its known layers follow bottom first unless it is collapsed. Layers
    /// outside the list appear as context and do not count towards the queue.
    pub fn build_all(
        list: &[PullRequest],
        collapsed: &CollapsedGroups,
        trees: &QueueTrees,
    ) -> Vec<Self> {
        // The repositories in the order they first appear.
        let repositories =
            distinct_repositories(list.iter().map(|pull_request| &pull_request.repo));

        // Each one's header, then its rows.
        let mut rows = Vec::new();
        for repo in repositories {
            let indexes = repository_indexes(list, repo);
            let is_collapsed = collapsed.repositories.contains(repo);
            rows.push(repo_header(repo, &indexes, is_collapsed, None));
            if !is_collapsed {
                rows.extend(repository_rows(list, &indexes, &collapsed.stacks, trees));
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
        trees: &QueueTrees,
    ) -> Vec<Self> {
        let mut rows = Vec::new();
        for repository in repositories {
            let repo = &repository.repo;
            let indexes = repository_indexes(shown, repo);
            let is_collapsed = collapsed.repositories.contains(repo);
            rows.push(repo_header(
                repo,
                &indexes,
                is_collapsed,
                Some(repository.group.clone()),
            ));
            if is_collapsed {
                continue;
            }
            if repository.group.is_filter_open {
                rows.push(Self::OthersFilter(repo.clone()));
            }
            if indexes.is_empty() {
                rows.push(Self::NothingShown(repo.clone()));
            } else {
                rows.extend(repository_rows(shown, &indexes, &collapsed.stacks, trees));
            }
        }
        rows
    }

    /// The index in the shown list of the pull request this row shows.
    pub fn pull_request_index(&self) -> Option<usize> {
        match self {
            Self::PullRequest(index) => Some(*index),
            Self::Layer { listed, .. } => *listed,
            Self::RepoHeader(_)
            | Self::StackHeader(_)
            | Self::OthersFilter(_)
            | Self::NothingShown(_) => None,
        }
    }
}

/// The indexes in `list` of `repo`'s pull requests, in list order.
fn repository_indexes(list: &[PullRequest], repo: &RepoRef) -> Vec<usize> {
    list.iter()
        .enumerate()
        .filter(|(_, pull_request)| &pull_request.repo == repo)
        .map(|(index, _)| index)
        .collect()
}

/// The header row of `repo`, whose pull requests are at `indexes` in the
/// shown list; `others` holds its filter in the Others tab.
fn repo_header(
    repo: &RepoRef,
    indexes: &[usize],
    is_collapsed: bool,
    others: Option<OthersGroup>,
) -> QueueRow {
    QueueRow::RepoHeader(RepoGroup {
        repo: repo.clone(),
        pull_requests: indexes.len(),
        is_collapsed,
        others,
    })
}

/// The rows of one repository's pull requests, at `indexes` in `list`: a
/// standalone pull request is one row; a stack is placed once, at its first
/// listed layer, as its header and (unless collapsed) its layers.
fn repository_rows(
    list: &[PullRequest],
    indexes: &[usize],
    collapsed_stacks: &HashSet<StackKey>,
    trees: &QueueTrees,
) -> Vec<QueueRow> {
    let mut rows = Vec::new();
    let mut placed = HashSet::new();
    let listed: HashMap<_, _> = indexes
        .iter()
        .map(|&index| (list[index].number, index))
        .collect();
    for &index in indexes {
        let pull_request = &list[index];
        let Some(key) = trees.key(&pull_request.repo, pull_request.number) else {
            rows.push(QueueRow::PullRequest(index));
            continue;
        };
        if !placed.insert(key) {
            continue;
        }
        let tree = trees.group(key).expect("indexed group");
        let is_collapsed = collapsed_stacks.contains(key);
        rows.push(QueueRow::StackHeader(StackGroup {
            key: key.clone(),
            size: tree.size,
            base_ref: tree.base_ref.clone(),
            listed: tree
                .entries
                .iter()
                .filter(|entry| listed.contains_key(&entry.number))
                .count(),
            is_collapsed,
            source: tree.source(),
            branching: tree.branching,
            root: tree.entries[0].number,
        }));
        if !is_collapsed {
            rows.extend(
                tree.entries
                    .iter()
                    .enumerate()
                    .map(|(entry, node)| QueueRow::Layer {
                        key: key.clone(),
                        entry,
                        listed: listed.get(&node.number).copied(),
                    }),
            );
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use qd_github::{PullRequestStack, RepoRef, StackLayer, StackMembership, User};

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
            branches: None,
            viewer_review: None,
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
        let rows = QueueRow::build_all(
            &list,
            &CollapsedGroups::default(),
            &QueueTrees::build(&list, &HashMap::new()),
        );
        assert!(matches!(&rows[0], QueueRow::RepoHeader(group) if group.pull_requests == 3));
        assert!(
            matches!(&rows[1], QueueRow::StackHeader(group) if group.listed == 2 && group.size == 3)
        );
        assert_eq!(
            rows[2..],
            [
                QueueRow::Layer {
                    key: StackKey::of(&list[2]).unwrap(),
                    entry: 0,
                    listed: Some(2)
                },
                QueueRow::Layer {
                    key: StackKey::of(&list[0]).unwrap(),
                    entry: 1,
                    listed: Some(0)
                },
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
        let rows = QueueRow::build_all(
            &list,
            &collapsed,
            &QueueTrees::build(&list, &HashMap::new()),
        );
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
        let rows = QueueRow::build_all(
            &list,
            &CollapsedGroups::default(),
            &QueueTrees::build(&list, &HashMap::new()),
        );
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
        let rows = QueueRow::build_all(
            &list,
            &collapsed,
            &QueueTrees::build(&list, &HashMap::new()),
        );
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
            head_repository: None,
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
            &QueueTrees::build(&shown, &HashMap::new()),
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

    /// A request for the middle layer shows both neighboring layers while counting just the request.
    #[test]
    fn requested_layer_shows_the_whole_stack() {
        let list = [pull_request(12, Some((9, 2)))];
        let key = StackKey::of(&list[0]).unwrap();
        let stack = PullRequestStack {
            repo: key.repo.clone(),
            number: 9,
            base_ref: "main".into(),
            is_open: true,
            layers: vec![layer(11), layer(12), layer(13)],
        };
        let stacks = HashMap::from([(key.clone(), stack)]);
        let collapsed = CollapsedGroups::default();
        let rows = QueueRow::build_all(&list, &collapsed, &QueueTrees::build(&list, &stacks));
        assert!(matches!(&rows[0], QueueRow::RepoHeader(group) if group.pull_requests == 1));
        assert!(
            matches!(&rows[1], QueueRow::StackHeader(group) if group.listed == 1 && group.size == 3)
        );
        assert_eq!(
            rows[2..],
            [
                QueueRow::Layer {
                    key: key.clone(),
                    entry: 0,
                    listed: None
                },
                QueueRow::Layer {
                    key: key.clone(),
                    entry: 1,
                    listed: Some(0)
                },
                QueueRow::Layer {
                    key,
                    entry: 2,
                    listed: None
                },
            ]
        );
    }
    fn branch(number: u64, head: &str, base: &str) -> PullRequest {
        let mut request = pull_request(number, None);
        request.branches = Some(qd_github::PullRequestBranches {
            head_ref: head.into(),
            base_ref: base.into(),
            head_repository: Some(request.repo.full_name()),
            default_branch: Some("trunk".into()),
            revision: Some(qd_github::PullRequestRevision {
                head_sha: format!("head-{number}"),
                base_sha: format!("base-{number}"),
            }),
        });
        request
    }

    /// The same PRs produce a series or a fork solely by changing the export PR's base.
    /// A single requested tip still brings both sibling subtrees into view exactly once.
    #[test]
    fn retargeting_changes_the_rail_and_only_the_selected_ancestry() {
        let mut known = [
            branch(30, "highlight", "search"),
            branch(10, "index", "trunk"),
            branch(40, "export", "trunk"),
            branch(20, "search", "index"),
        ];
        let trees = QueueTrees::build(&known, &HashMap::new());
        let key = trees.key(&known[0].repo, 30).unwrap();
        let series = trees.group(key).unwrap();
        assert!(!series.branching);
        assert!(
            series
                .entries
                .iter()
                .all(|entry| entry.rail.lanes.is_empty())
        );
        assert!(trees.key(&known[0].repo, 40).is_none());
        known[2].branches.as_mut().unwrap().base_ref = "index".into();
        let trees = QueueTrees::build(known.iter().chain([&known[0]]), &HashMap::new());
        let key = trees.key(&known[0].repo, 30).unwrap();
        let tree = trees.group(key).unwrap();
        assert!(tree.branching);
        assert_eq!(
            tree.entries
                .iter()
                .map(|entry| entry.number)
                .collect::<Vec<_>>(),
            [10, 20, 30, 40]
        );
        assert_eq!(tree.entries[1].rail.lanes, [true]);
        assert_eq!(tree.entries[2].rail.lanes, [true, false]);
        assert_eq!(tree.entries[3].rail.lanes, [false]);
        let rows = QueueRow::build_all(&known[..1], &CollapsedGroups::default(), &trees);
        assert!(
            matches!(&rows[1],QueueRow::StackHeader(group) if group.listed == 1 && group.size == 4)
        );
        assert_eq!(rows.len(), 6);
        for (request, expected) in [
            (&known[0], vec![10, 20, 30]),
            (&known[2], vec![10, 40]),
            (&known[1], vec![10]),
        ] {
            assert_eq!(
                trees
                    .review_path(request)
                    .unwrap()
                    .iter()
                    .map(|layer| layer.number)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }

    /// Ambiguous heads, another fork, deleted heads, trunk and cycles provide no parent.
    #[test]
    fn unsafe_branch_matches_stay_ungrouped() {
        let mut known = [
            branch(1, "x", "trunk"),
            branch(2, "x", "trunk"),
            branch(3, "child", "x"),
            branch(4, "fork-only", "trunk"),
            branch(5, "child2", "fork-only"),
            branch(6, "trunk", "release"),
            branch(7, "cycle-left", "cycle-right"),
            branch(8, "cycle-right", "cycle-left"),
            branch(9, "deleted", "trunk"),
            branch(10, "child3", "deleted"),
        ];
        known[3].branches.as_mut().unwrap().head_repository = Some("fork/b".into());
        known[8].branches.as_mut().unwrap().head_repository = None;
        let trees = QueueTrees::build(&known, &HashMap::new());
        for request in &known {
            assert!(trees.key(&request.repo, request.number).is_none());
        }
    }

    /// A changed native order can arrive via the PR lists before the stack endpoint.
    #[test]
    fn conflicting_snapshots_do_not_splice_two_prs_into_the_same_native_slot() {
        let listed = [
            pull_request(10, Some((99, 1))),
            pull_request(21, Some((99, 2))),
        ];
        let key = StackKey::of(&listed[0]).unwrap();
        let mut stack = PullRequestStack {
            repo: key.repo.clone(),
            number: 99,
            base_ref: "main".into(),
            is_open: true,
            layers: vec![layer(10), layer(20)],
        };
        for expected in [vec![10, 21], vec![10, 21, 20]] {
            let trees = QueueTrees::build(&listed, &HashMap::from([(key.clone(), stack.clone())]));
            assert_eq!(
                trees
                    .group(&key)
                    .unwrap()
                    .entries
                    .iter()
                    .map(|entry| entry.number)
                    .collect::<Vec<_>>(),
                expected
            );
            stack.layers = vec![layer(10), layer(21), layer(20)];
        }
    }

    /// An unlisted native root still owns its branch. A branch-matched sibling uses
    /// that root's commits and never acquires native membership.
    #[test]
    fn inferred_child_can_branch_off_a_native_context_layer() {
        let native = pull_request(20, Some((99, 2)));
        let child = branch(40, "export", "index");
        let key = StackKey::of(&native).unwrap();
        let mut root = layer(10);
        root.head_ref = "index".into();
        root.head_repository = Some(native.repo.full_name());
        root.base_sha = "trunk-commit".into();
        let stack = PullRequestStack {
            repo: native.repo.clone(),
            number: 99,
            base_ref: "trunk".into(),
            is_open: true,
            layers: vec![root, layer(20)],
        };
        let trees = QueueTrees::build([&native, &child], &HashMap::from([(key.clone(), stack)]));
        let group = trees.group(&key).unwrap();
        assert!(group.has_native && group.inferred && group.branching);
        let path = trees.review_path(&child).unwrap();
        assert_eq!(
            path.iter().map(|entry| entry.number).collect::<Vec<_>>(),
            [10, 40]
        );
        assert_eq!(path[0].base_sha, "trunk-commit");
        assert_eq!(path[1].head_sha, "head-40");
        assert!(child.stack.is_none());
    }
}
