use std::collections::{BTreeMap, HashMap, HashSet};

use qd_github::{PullRequest, PullRequestStack, RepoRef, StackLayer};

use super::stack_key::StackKey;

/// The parent links shared by all three tabs. A tab chooses which groups to show;
/// it never changes their topology by filtering out a sibling.
#[derive(Default)]
pub(super) struct QueueTrees {
    groups: HashMap<StackKey, QueueTree>,
    locations: HashMap<RepoRef, HashMap<u64, (StackKey, usize)>>,
}

/// A connected group in sidebar order, including its context rows.
pub(super) struct QueueTree {
    pub entries: Vec<TreeEntry>,
    pub base_ref: String,
    pub size: u64,
    pub inferred: bool,
    pub has_native: bool,
    pub branching: bool,
    pub max_depth: usize,
}

impl QueueTree {
    /// The actual edge behind a queue row, including provenance in a mixed tree.
    pub fn parent_hint(&self, index: usize) -> Option<String> {
        let entry = self.entries.get(index)?;
        if let Some(stack) = entry.native_stack
            && !entry.inferred_parent
        {
            return Some(format!("GitHub stack #{stack}"));
        }
        let parent = &self.entries[entry.parent?];
        let branch = &entry.layer.as_ref()?.base_ref;
        Some(format!(
            "Branch match · {branch} is the head of #{}",
            parent.number
        ))
    }

    pub fn source(&self) -> super::stack_group::StackSource {
        use super::stack_group::StackSource;
        match (self.inferred, self.has_native) {
            (true, true) => StackSource::Mixed,
            (true, false) => StackSource::BranchMatches,
            (false, _) => StackSource::GitHub,
        }
    }
}

/// A PR in depth-first queue order, with its native location when one exists.
pub(super) struct TreeEntry {
    pub number: u64,
    /// A native layer outside every list can still be opened by its true stack position.
    pub native: Option<(StackKey, usize)>,
    parent: Option<usize>,
    pub children: usize,
    subtree_end: usize,
    native_stack: Option<u64>,
    inferred_parent: bool,
    pub rail: LayerRail,
    layer: Option<StackLayer>,
    missing_parent: bool,
}

/// One row's part of the rail. Each fork lane says whether another sibling follows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct LayerRail {
    pub lanes: Vec<bool>,
    pub straight_in: bool,
    pub straight_out: bool,
}

#[derive(Default)]
struct Node<'a> {
    unseen_layers: u64,
    listed: Option<&'a PullRequest>,
    native: Option<(&'a StackKey, &'a PullRequestStack, usize)>,
    parent: Option<u64>,
    inferred: bool,
    missing_parent: bool,
}

impl Node<'_> {
    /// The authoritative stack slot from the full response or its listed membership.
    fn native_position(&self) -> Option<(u64, u64)> {
        if let Some((key, _, index)) = self.native {
            return Some((key.number, u64::try_from(index).ok()? + 1));
        }
        let member = self.listed?.stack.as_ref()?;
        Some((member.number, member.position))
    }

    fn native_layer(&self) -> Option<&StackLayer> {
        let (_, stack, index) = self.native?;
        stack.layers.get(index)
    }

    /// Only an open PR with a head in this repository can parent another local PR.
    fn local_head(&self, repository: &str) -> Option<&str> {
        let (name, owner) = if let Some(layer) = self.native_layer() {
            if !layer.is_open {
                return None;
            }
            (
                &layer.head_ref,
                layer.head_repository.as_deref().or_else(|| {
                    self.listed?
                        .branches
                        .as_ref()
                        .filter(|branches| branches.head_ref == layer.head_ref)?
                        .head_repository
                        .as_deref()
                }),
            )
        } else {
            let branches = self.listed?.branches.as_ref()?;
            (&branches.head_ref, branches.head_repository.as_deref())
        };
        owner?.eq_ignore_ascii_case(repository).then_some(name)
    }

    /// The branch names and pinned commits consumed by combined reviews.
    fn layer(&self) -> Option<StackLayer> {
        if let Some(layer) = self.native_layer() {
            return Some(layer.clone());
        }
        let pull_request = self.listed?;
        let branches = pull_request.branches.as_ref()?;
        let revision = branches.revision.as_ref()?;
        Some(StackLayer {
            pull_request_id: pull_request.id,
            number: pull_request.number,
            title: pull_request.title.clone(),
            head_repository: branches.head_repository.clone(),
            head_ref: branches.head_ref.clone(),
            base_ref: branches.base_ref.clone(),
            head_sha: revision.head_sha.clone(),
            base_sha: revision.base_sha.clone(),
            is_open: true,
            is_merged: false,
            is_draft: pull_request.draft,
            html_url: pull_request.html_url.clone(),
            author_login: pull_request.author.login.clone(),
        })
    }
}

impl QueueTrees {
    /// Prefer the newest observation of a PR shared by several lists. Include all layers
    /// of the native stacks still represented in those lists, even if only one is requested.
    pub fn build<'a>(
        listed: impl IntoIterator<Item = &'a PullRequest>,
        stacks: &'a HashMap<StackKey, PullRequestStack>,
    ) -> Self {
        let mut repos: HashMap<&RepoRef, BTreeMap<u64, Node>> = HashMap::new();
        for pull_request in listed {
            let node = repos
                .entry(&pull_request.repo)
                .or_default()
                .entry(pull_request.number)
                .or_default();
            if node
                .listed
                .is_none_or(|old| old.updated_at < pull_request.updated_at)
            {
                node.listed = Some(pull_request);
            }
        }
        // A list refresh can arrive before the new native stack. Splicing the two
        // orders together could give a PR a sibling's base in a combined review.
        let mut listed_slots: HashMap<StackKey, Vec<(u64, u64)>> = HashMap::new();
        for pr in repos
            .values()
            .flat_map(|nodes| nodes.values())
            .filter_map(|node| node.listed)
        {
            if let Some(member) = &pr.stack {
                let key = StackKey::native(&pr.repo, member.number);
                listed_slots
                    .entry(key)
                    .or_default()
                    .push((member.position, pr.number));
            }
        }
        for (listed_key, slots) in listed_slots {
            let Some((key, stack)) = stacks.get_key_value(&listed_key) else {
                continue;
            };
            if !slots.iter().all(|&(position, number)| {
                let index = position
                    .checked_sub(1)
                    .and_then(|slot| usize::try_from(slot).ok());
                index
                    .and_then(|index| stack.layers.get(index))
                    .is_some_and(|layer| layer.number == number)
            }) {
                continue;
            }
            let nodes = repos.entry(&key.repo).or_default();
            for (index, layer) in stack.layers.iter().enumerate() {
                let node = nodes.entry(layer.number).or_default();
                if node.listed.is_none_or(|listed| {
                    listed
                        .stack
                        .as_ref()
                        .is_some_and(|member| member.number == key.number)
                }) {
                    node.native = Some((key, stack, index));
                }
            }
        }
        let mut trees = Self::default();
        for (repo, mut nodes) in repos {
            connect(repo, &mut nodes);
            trees.add_groups(repo, &nodes);
        }
        trees
    }

    pub fn key(&self, repo: &RepoRef, number: u64) -> Option<&StackKey> {
        Some(&self.locations.get(repo)?.get(&number)?.0)
    }

    pub fn group(&self, key: &StackKey) -> Option<&QueueTree> {
        self.groups.get(key)
    }

    pub fn entry(&self, repo: &RepoRef, number: u64) -> Option<&TreeEntry> {
        let (key, index) = self.locations.get(repo)?.get(&number)?;
        self.groups.get(key)?.entries.get(*index)
    }

    /// Depth-first ordering makes a node's subtree one interval, so painting a
    /// visible row only needs two comparisons to highlight the selected ancestry.
    pub fn on_path(&self, key: &StackKey, index: usize, selection: &PullRequest) -> bool {
        let Some((group, selected)) = self
            .locations
            .get(&selection.repo)
            .and_then(|locations| locations.get(&selection.number))
        else {
            return false;
        };
        group == key && (index..=self.groups[key].entries[index].subtree_end).contains(selected)
    }

    /// Native-only groups keep using the native stack loader. A mixed or inferred
    /// group supplies only ancestors and unambiguous successors for layer navigation.
    pub fn review_path(&self, pull_request: &PullRequest) -> Option<Vec<StackLayer>> {
        let (key, index) = self
            .locations
            .get(&pull_request.repo)?
            .get(&pull_request.number)?;
        let group = self.group(key)?;
        if !group.inferred {
            return None;
        }
        let mut path = Vec::new();
        let mut ancestor = Some(*index);
        while let Some(index) = ancestor {
            let entry = &group.entries[index];
            if entry.missing_parent {
                return None;
            }
            path.push(entry.layer.clone()?);
            ancestor = entry.parent;
        }
        path.reverse();
        // A fork has no unique next layer; the sidebar lets the reviewer choose.
        let mut current = *index;
        while let Some(next) = group.entries.get(current + 1) {
            if group.entries[current].children != 1 || next.missing_parent {
                break;
            }
            let Some(layer) = next.layer.clone() else {
                break;
            };
            path.push(layer);
            current += 1;
        }
        Some(path)
    }

    /// Flatten each component once, retaining ancestry, counts and rail geometry.
    fn add_groups(&mut self, repo: &RepoRef, nodes: &BTreeMap<u64, Node>) {
        let mut children: HashMap<u64, Vec<u64>> = HashMap::new();
        for (&number, node) in nodes {
            if let Some(parent) = node.parent {
                children.entry(parent).or_default().push(number);
            }
        }
        for (&root, node) in nodes.iter().filter(|(_, node)| node.parent.is_none()) {
            let native = node.native_position();
            if node.unseen_layers == 0 && !children.contains_key(&root) {
                continue;
            }
            let key = StackKey {
                repo: repo.clone(),
                number: native.map_or(root, |p| p.0),
                branch_root: native.is_none(),
            };
            // In a connected tree, following only children reaches either the sole
            // tip or the first fork. Only the latter needs indentation.
            let mut tip = root;
            while let Some([child]) = children.get(&tip).map(Vec::as_slice) {
                tip = *child;
            }
            let branching = children.contains_key(&tip);
            let mut tree = QueueTree {
                entries: Vec::new(),
                size: 0,
                inferred: false,
                has_native: false,
                branching,
                max_depth: 0,
                base_ref: node
                    .native
                    .map(|(_, stack, _)| stack.base_ref.clone())
                    .or_else(|| {
                        node.listed?
                            .stack
                            .as_ref()
                            .map(|member| member.base_ref.clone())
                    })
                    .or_else(|| {
                        node.listed?
                            .branches
                            .as_ref()
                            .map(|branches| branches.base_ref.clone())
                    })
                    .unwrap_or_default(),
            };
            let mut pending = vec![(root, None, Vec::new())];

            let locations = self.locations.entry(repo.clone()).or_default();
            // Visit each parent first, then finish its subtree before the next sibling.
            while let Some((number, parent, lanes)) = pending.pop() {
                let node = &nodes[&number];
                let index = tree.entries.len();
                locations.insert(number, (key.clone(), index));
                tree.inferred |= node.inferred;
                tree.has_native |= node.native_position().is_some();
                tree.size = tree
                    .size
                    .saturating_add(1)
                    .saturating_add(node.unseen_layers);
                let descendants = children.get(&number).map(Vec::as_slice).unwrap_or_default();
                for (child_index, child) in descendants.iter().enumerate().rev() {
                    let mut child_lanes = lanes.clone();
                    if branching {
                        child_lanes.push(child_index + 1 < descendants.len());
                    }
                    pending.push((*child, Some(index), child_lanes));
                }
                tree.max_depth = tree.max_depth.max(lanes.len());
                tree.entries.push(TreeEntry {
                    inferred_parent: node.inferred,
                    native_stack: node.native_position().map(|position| position.0),
                    number,
                    parent,
                    children: descendants.len(),
                    subtree_end: index,
                    layer: node.layer(),
                    native: node.native.map(|(key, _, index)| (key.clone(), index)),
                    missing_parent: node.missing_parent,
                    rail: LayerRail {
                        lanes,
                        straight_in: !branching && parent.is_some(),
                        straight_out: !descendants.is_empty(),
                    },
                });
            }
            // Each subtree occupies a contiguous range, including all its descendants.
            for index in (0..tree.entries.len()).rev() {
                if let Some(parent) = tree.entries[index].parent {
                    tree.entries[parent].subtree_end = tree.entries[parent]
                        .subtree_end
                        .max(tree.entries[index].subtree_end);
                }
            }
            self.groups.insert(key, tree);
        }
    }
}

/// Native membership fixes the order first. A branch match adds a parent only
/// when a unique known open head owns that branch and the base is not the default.
fn connect(repo: &RepoRef, nodes: &mut BTreeMap<u64, Node>) {
    let mut native: HashMap<u64, Vec<(u64, u64)>> = HashMap::new();
    let mut heads: HashMap<String, Option<u64>> = HashMap::new();
    let repository = repo.full_name();
    for (&number, node) in nodes.iter() {
        if let Some((stack, position)) = node.native_position() {
            native.entry(stack).or_default().push((position, number));
        }
        if let Some(head) = node.local_head(&repository) {
            heads
                .entry(head.to_owned())
                .and_modify(|candidate| *candidate = None)
                .or_insert(Some(number));
        }
    }
    for members in native.values_mut() {
        members.sort_unstable();
        // Before the full stack arrives, its header still counts the missing layers.
        if !members
            .iter()
            .any(|(_, number)| nodes[number].native.is_some())
        {
            let size = members
                .iter()
                .filter_map(|(_, number)| {
                    nodes[number]
                        .listed?
                        .stack
                        .as_ref()
                        .map(|member| member.size)
                })
                .max()
                .unwrap_or(0);
            if let Some((_, root)) = members.first() {
                nodes
                    .get_mut(root)
                    .expect("listed stack root")
                    .unseen_layers =
                    size.saturating_sub(u64::try_from(members.len()).unwrap_or(u64::MAX));
            }
        }
        let mut previous = None;
        for &(position, number) in members.iter() {
            let node = nodes.get_mut(&number).expect("indexed native layer");
            node.parent = previous.map(|(_, number)| number);
            node.missing_parent = previous.map_or(position != 1, |(last, _)| {
                Some(position) != u64::checked_add(last, 1)
            });
            previous = Some((position, number));
        }
    }
    // The base repository is common to these PRs. Its default branch can also
    // bound a native root known only through the stack response.
    let default_branch = nodes
        .values()
        .find_map(|node| node.listed?.branches.as_ref()?.default_branch.clone());
    for (&number, node) in nodes
        .iter_mut()
        .filter(|(_, node)| node.parent.is_none() && !node.missing_parent)
    {
        let base = match node.native_layer() {
            Some(layer) if layer.is_open => Some(layer.base_ref.as_str()),
            Some(_) => None,
            None => node
                .listed
                .and_then(|pr| pr.branches.as_ref())
                .map(|branches| branches.base_ref.as_str()),
        };
        let Some(base) = base else {
            continue;
        };
        if default_branch
            .as_deref()
            .is_none_or(|default| default == base)
        {
            continue;
        }
        if let Some(&Some(parent)) = heads.get(base)
            && parent != number
        {
            node.parent = Some(parent);
            node.inferred = true;
        }
    }
    remove_cyclic_matches(nodes);
}

/// Discard all inferred edges on a cycle. Each node is visited once even for long chains.
fn remove_cyclic_matches(nodes: &mut BTreeMap<u64, Node>) {
    // A cycle invalidates each inferred edge on it, regardless of traversal order.
    let mut finished = HashSet::new();
    for start in nodes.keys().copied().collect::<Vec<_>>() {
        let mut path = Vec::new();
        let mut positions = HashMap::new();
        let mut cursor = Some(start);
        while let Some(number) = cursor {
            if finished.contains(&number) {
                break;
            }
            if let Some(index) = positions.insert(number, path.len()) {
                for member in &path[index..] {
                    let node = nodes.get_mut(member).expect("visited node");
                    if node.inferred {
                        node.parent = None;
                        node.inferred = false;
                    }
                }
                break;
            }
            path.push(number);
            cursor = nodes[&number].parent;
        }
        finished.extend(path);
    }
}
