use std::collections::HashSet;

use qc_github::PullRequest;

use super::stack_key::StackKey;

/// One row of the queue's sidebar list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueRow {
    /// A standalone pull request, by its index in the shown list.
    PullRequest(usize),
    /// The header above a stack's listed layers.
    StackHeader(StackGroup),
    /// A listed layer of the stack above, by its index in the shown list.
    StackLayer(usize),
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
    /// The rows for `list`, in its order (most recently updated first):
    /// a stack's header takes the place of its first listed layer and its
    /// listed layers follow bottom first, unless it is collapsed.
    pub fn build_all(list: &[PullRequest], collapsed: &HashSet<StackKey>) -> Vec<Self> {
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

            // Its layers in the list, bottom first.
            let mut layers: Vec<usize> = list
                .iter()
                .enumerate()
                .filter(|(_, candidate)| StackKey::of(candidate).as_ref() == Some(&key))
                .map(|(layer_index, _)| layer_index)
                .collect();
            layers.sort_by_key(|layer_index| {
                list[*layer_index]
                    .stack
                    .as_ref()
                    .map_or(0, |membership| membership.position)
            });
            let is_collapsed = collapsed.contains(&key);
            rows.push(Self::StackHeader(StackGroup {
                size: membership.size,
                base_ref: membership.base_ref.clone(),
                listed: layers.len(),
                is_collapsed,
                key,
            }));
            if !is_collapsed {
                rows.extend(layers.into_iter().map(Self::StackLayer));
            }
        }
        rows
    }

    /// The index in the shown list of the pull request this row shows.
    pub fn pull_request_index(&self) -> Option<usize> {
        match self {
            Self::PullRequest(index) | Self::StackLayer(index) => Some(*index),
            Self::StackHeader(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use qc_github::{RepoRef, StackMembership, User};

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
        let rows = QueueRow::build_all(&list, &HashSet::new());
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
        let rows = QueueRow::build_all(&list, &HashSet::from([key]));
        assert_eq!(rows.len(), 1);
        assert!(matches!(&rows[0], QueueRow::StackHeader(group) if group.is_collapsed));
    }
}
