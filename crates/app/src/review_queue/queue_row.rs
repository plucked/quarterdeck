use std::collections::{HashMap, HashSet};

use qd_github::{PullRequest, PullRequestStack};

use super::stack_key::StackKey;

/// One row of the queue's sidebar list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueRow {
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
    /// The rows for `list`, in its order (most recently updated first): a
    /// stack's header takes the place of its first listed layer, and its
    /// layers follow bottom first unless it is collapsed. With the stack's
    /// layers in `stacks`, every layer shows, listed or not; without, only
    /// the listed ones.
    pub fn build_all(
        list: &[PullRequest],
        collapsed: &HashSet<StackKey>,
        stacks: &HashMap<StackKey, PullRequestStack>,
    ) -> Vec<Self> {
        let mut rows = Vec::new();
        let mut placed: HashSet<StackKey> = HashSet::new();
        for (index, pull_request) in list.iter().enumerate() {
            // A standalone pull request is one row; a stack is placed once, at its first layer.
            let (Some(key), Some(membership)) = (StackKey::of(pull_request), &pull_request.stack)
            else {
                rows.push(Self::PullRequest(index));
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
                None => listed.iter().copied().map(Self::StackLayer).collect(),
            };
            let is_collapsed = collapsed.contains(&key);
            rows.push(Self::StackHeader(StackGroup {
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

    /// The index in the shown list of the pull request this row shows.
    pub fn pull_request_index(&self) -> Option<usize> {
        match self {
            Self::PullRequest(index) | Self::StackLayer(index) => Some(*index),
            Self::StackHeader(_) | Self::OtherLayer { .. } => None,
        }
    }
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
        PullRequest {
            id: number,
            number,
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
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
        let rows = QueueRow::build_all(&list, &HashSet::new(), &HashMap::new());
        assert!(
            matches!(&rows[0], QueueRow::StackHeader(group) if group.listed == 2 && group.size == 3)
        );
        assert_eq!(
            rows[1..],
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
        let key = StackKey::of(&list[0]).unwrap();
        let rows = QueueRow::build_all(&list, &HashSet::from([key]), &HashMap::new());
        assert_eq!(rows.len(), 1);
        assert!(matches!(&rows[0], QueueRow::StackHeader(group) if group.is_collapsed));
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
            &HashSet::new(),
            &HashMap::from([(key.clone(), stack)]),
        );
        assert!(
            matches!(&rows[0], QueueRow::StackHeader(group) if group.listed == 1 && group.size == 3)
        );
        assert_eq!(
            rows[1..],
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
