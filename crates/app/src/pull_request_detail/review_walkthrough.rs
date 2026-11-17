use std::collections::{HashMap, HashSet};

use gpui::{ListAlignment, ListState, px};
use qd_studio::{SmartFilter, SmartReviewItem};

use super::file_review_state::FileReviewState;
use super::loaded_file::LoadedFile;
use super::walkthrough_bucket::WalkthroughBucket;
use super::walkthrough_item::WalkthroughItem;
use super::walkthrough_item_id::WalkthroughItemId;
use super::walkthrough_row::WalkthroughRow;

/// Derived plan navigation, rebuilt on data changes rather than during rendering.
pub(super) struct ReviewWalkthrough {
    pub buckets: Vec<WalkthroughBucket>,
    pub rows: Vec<WalkthroughRow>,
    pub open_bucket: Option<u32>,
    pub expanded_items: HashSet<WalkthroughItemId>,
    pub current_item: Option<WalkthroughItemId>,
    pub selected_group: Option<WalkthroughItemId>,
    pub guidance: HashSet<u32>,
    pub list: ListState,
    pub group_diff: super::group_diff::GroupDiff,
}

impl Default for ReviewWalkthrough {
    fn default() -> Self {
        Self {
            buckets: Vec::new(),
            rows: Vec::new(),
            open_bucket: Some(1),
            expanded_items: HashSet::new(),
            current_item: None,
            selected_group: None,
            guidance: HashSet::new(),
            list: ListState::new(0, ListAlignment::Top, px(300.)),
            group_diff: super::group_diff::GroupDiff::default(),
        }
    }
}

impl ReviewWalkthrough {
    /// Maps exact saved membership to current files; newly changed files get an explicit unplanned bucket.
    pub fn refresh(
        &mut self,
        plan: Option<&SmartFilter>,
        files: &[LoadedFile],
        shown: &[bool],
        review: &FileReviewState,
    ) {
        let mut buckets = Vec::new();
        let paths: HashMap<_, _> = files
            .iter()
            .enumerate()
            .map(|(index, file)| (file.filename(), index))
            .collect();
        let mut covered = HashSet::new();
        if let Some(plan) = plan {
            for bucket in &plan.buckets {
                let items = &bucket.items;
                let prepared = items
                    .iter()
                    .filter_map(|item| {
                        let members: Vec<_> = item
                            .files
                            .iter()
                            .filter_map(|path| paths.get(path.as_str()).copied())
                            .collect();
                        if members.is_empty() {
                            return None;
                        }
                        covered.extend(members.iter().copied());
                        Some(prepare_item(item, members, files, shown, review))
                    })
                    .collect();
                buckets.push(prepare_bucket(
                    bucket.order,
                    bucket.label.clone(),
                    bucket.description.clone(),
                    prepared,
                ));
            }
            let unplanned = files
                .iter()
                .enumerate()
                .filter(|(index, _)| !covered.contains(index))
                .map(|(index, file)| {
                    prepare_item(
                        &SmartReviewItem {
                            label: file.filename().into(),
                            description: "Refresh the guide to suggest an order.".into(),
                            files: vec![],
                        },
                        vec![index],
                        files,
                        shown,
                        review,
                    )
                })
                .collect::<Vec<_>>();
            if !unplanned.is_empty() {
                buckets.push(prepare_bucket(
                    0,
                    "Ungrouped files".into(),
                    "These changed files are outside the saved guide.".into(),
                    unplanned,
                ));
            }
        }
        self.buckets = buckets;
        self.refresh_visible_rows(shown);
    }

    /// Rebuilds only navigation rows from the prepared bucket contents.
    fn refresh_visible_rows(&mut self, shown: &[bool]) {
        self.rows.clear();
        for (bucket_index, bucket) in self.buckets.iter().enumerate() {
            if !bucket.items.iter().any(|item| item.shown > 0) {
                continue;
            }
            self.rows.push(WalkthroughRow::Bucket(bucket_index));
            if self.open_bucket != Some(bucket.order) {
                continue;
            }
            self.rows.push(WalkthroughRow::Actions(bucket_index));
            if self.guidance.contains(&bucket.order) {
                self.rows.push(WalkthroughRow::Guidance(bucket_index));
            }
            for (item_index, item) in bucket.items.iter().enumerate() {
                if item.shown == 0 {
                    continue;
                }
                let id = WalkthroughItemId {
                    bucket: bucket_index,
                    item: item_index,
                };
                self.rows.push(WalkthroughRow::Item(id));
                if item.files.len() > 1 {
                    self.rows.push(WalkthroughRow::GroupMembers(id));
                    if self.expanded_items.contains(&id) {
                        self.rows.extend(
                            item.files
                                .iter()
                                .filter(|file| shown[**file])
                                .map(|file| WalkthroughRow::File { id, file: *file }),
                        );
                    }
                }
            }
        }
        // Preserve the scroll offset while marks and filters change the rows.
        let mut top = self.list.logical_scroll_top();
        self.list.reset(self.rows.len());
        if !self.rows.is_empty() {
            top.item_ix = top.item_ix.min(self.rows.len() - 1);
            self.list.scroll_to(top);
        }
    }

    pub fn item(&self, id: WalkthroughItemId) -> Option<&WalkthroughItem> {
        self.buckets.get(id.bucket)?.items.get(id.item)
    }

    /// Resumes a valid selection, otherwise starts at unfinished visible work in guided order.
    pub fn entry_item(&self) -> Option<WalkthroughItemId> {
        // Filters and completion do not evict a step the reader already chose.
        if let Some(selected) = self.selected_group
            && self.item(selected).is_some()
        {
            return Some(selected);
        }
        let mut visible = self
            .buckets
            .iter()
            .enumerate()
            .flat_map(|(bucket, contents)| {
                contents
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(move |(item, contents)| {
                        (contents.shown > 0).then_some(WalkthroughItemId { bucket, item })
                    })
            });
        // A completed guide remains readable when no unfinished visible step remains.
        visible
            .clone()
            .find(|id| self.item(*id).is_some_and(|item| item.unviewed_shown > 0))
            .or_else(|| visible.next())
    }

    /// Advances across buckets, then wraps to earlier unfinished stops; never returns the current item.
    pub fn next_after(&self, current: WalkthroughItemId) -> Option<WalkthroughItemId> {
        // Preserve the suggested bucket and item order while skipping hidden or completed stops.
        let incomplete = self
            .buckets
            .iter()
            .enumerate()
            .flat_map(|(bucket, contents)| {
                contents
                    .items
                    .iter()
                    .enumerate()
                    .filter_map(move |(item, contents)| {
                        (contents.unviewed_shown > 0).then_some(WalkthroughItemId { bucket, item })
                    })
            });
        let is_after = |id: &WalkthroughItemId| {
            id.bucket > current.bucket || (id.bucket == current.bucket && id.item > current.item)
        };
        // Exhaust later buckets before returning to an unfinished earlier item.
        incomplete.clone().find(is_after).or_else(|| {
            incomplete
                .filter(|id| *id != current)
                .find(|id| !is_after(id))
        })
    }

    /// The next visible incomplete stop; a grouped item is visited only once.
    pub fn next(&self, bucket: usize) -> Option<WalkthroughItemId> {
        let items = &self.buckets.get(bucket)?.items;
        let start = self
            .current_item
            .filter(|current| current.bucket == bucket)
            .map_or(0, |current| current.item + 1)
            .min(items.len());
        // Move forward in the suggested order, wrapping to unfinished earlier stops.
        (start..items.len())
            .chain(0..start)
            .find(|index| items[*index].unviewed_shown > 0)
            .map(|item| WalkthroughItemId { bucket, item })
    }
}

/// Resolves displayed and viewed counts across the complete current membership.
fn prepare_item(
    item: &SmartReviewItem,
    members: Vec<usize>,
    files: &[LoadedFile],
    shown: &[bool],
    review: &FileReviewState,
) -> WalkthroughItem {
    WalkthroughItem {
        label: item.label.clone(),
        description: item.description.clone(),
        viewed: members
            .iter()
            .filter(|file| review.is_viewed(files[**file].filename()))
            .count(),
        unviewed_shown: members
            .iter()
            .filter(|file| shown[**file] && !review.is_viewed(files[**file].filename()))
            .count(),
        shown: members.iter().filter(|file| shown[**file]).count(),
        files: members,
    }
}

/// Summarizes file progress separately from walkthrough stop progress.
fn prepare_bucket(
    order: u32,
    label: String,
    description: String,
    items: Vec<WalkthroughItem>,
) -> WalkthroughBucket {
    WalkthroughBucket {
        order,
        label,
        description,
        remaining: items
            .iter()
            .filter(|item| item.viewed < item.files.len())
            .count(),
        viewed: items.iter().map(|item| item.viewed).sum(),
        files: items
            .iter()
            .flat_map(|item| item.files.iter().copied())
            .collect(),
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;
    use qd_studio::SmartBucket;

    /// Entering Guided review preserves its selected step, then prefers unfinished visible work.
    #[test]
    fn guided_entry_preserves_selection_and_falls_back_in_order() {
        let make_item = |shown, unviewed_shown| WalkthroughItem {
            label: String::new(),
            description: String::new(),
            files: vec![0],
            viewed: usize::from(unviewed_shown == 0),
            shown,
            unviewed_shown,
        };
        let mut walkthrough = ReviewWalkthrough {
            buckets: vec![
                prepare_bucket(
                    1,
                    String::new(),
                    String::new(),
                    vec![make_item(1, 0), make_item(0, 0)],
                ),
                prepare_bucket(2, String::new(), String::new(), vec![make_item(1, 1)]),
            ],
            ..ReviewWalkthrough::default()
        };
        let first = WalkthroughItemId { bucket: 0, item: 0 };
        let hidden = WalkthroughItemId { bucket: 0, item: 1 };
        let unfinished = WalkthroughItemId { bucket: 1, item: 0 };

        // Explicit selections survive completion and filtering; invalid old selections do not.
        assert_eq!(walkthrough.entry_item(), Some(unfinished));
        walkthrough.selected_group = Some(hidden);
        assert_eq!(walkthrough.entry_item(), Some(hidden));
        walkthrough.selected_group = Some(WalkthroughItemId { bucket: 5, item: 0 });
        assert_eq!(walkthrough.entry_item(), Some(unfinished));

        // Once the guide is complete, the first visible step is available for rereading.
        walkthrough.buckets[1].items[0].unviewed_shown = 0;
        assert_eq!(walkthrough.entry_item(), Some(first));
        walkthrough.buckets[0].items[0].shown = 0;
        walkthrough.buckets[1].items[0].shown = 0;
        assert_eq!(walkthrough.entry_item(), None);
    }

    /// Finishing a group crosses buckets, skips hidden and completed stops, then wraps only to unfinished work.
    #[test]
    fn guided_order_advances_across_buckets_and_stops_when_complete() {
        // Mix unfinished, completed and filtered items in two ordered buckets.
        let make_item = |unviewed_shown| WalkthroughItem {
            label: String::new(),
            description: String::new(),
            files: vec![0, 1],
            viewed: 0,
            shown: unviewed_shown,
            unviewed_shown,
        };
        let mut walkthrough = ReviewWalkthrough {
            buckets: vec![
                prepare_bucket(
                    1,
                    String::new(),
                    String::new(),
                    vec![make_item(1), make_item(0)],
                ),
                prepare_bucket(
                    2,
                    String::new(),
                    String::new(),
                    vec![make_item(0), make_item(1)],
                ),
            ],
            ..ReviewWalkthrough::default()
        };
        let first = WalkthroughItemId { bucket: 0, item: 0 };
        let group = WalkthroughItemId { bucket: 0, item: 1 };
        let last = WalkthroughItemId { bucket: 1, item: 1 };

        // Later buckets take precedence over earlier unfinished items.
        assert_eq!(walkthrough.next_after(group), Some(last));
        assert_eq!(walkthrough.next_after(last), Some(first));
        walkthrough.buckets[1].items[1].unviewed_shown = 0;
        assert_eq!(walkthrough.next_after(group), Some(first));

        // With only the current stop left, Next cannot jump back to itself.
        assert_eq!(walkthrough.next_after(first), None);
        walkthrough.buckets[0].items[0].unviewed_shown = 0;
        assert_eq!(walkthrough.next_after(group), None);
    }

    /// A filtered group retains its complete membership, skips as one completed item, and leaves new files unplanned.
    #[test]
    fn filtered_stale_plan_keeps_group_membership_and_progress() {
        let paths = ["format.md", "fixtures/a.bin", "fixtures/b.bin", "new.rs"];
        let files: Vec<_> = paths
            .iter()
            .map(|path| {
                LoadedFile::build(
                    PullRequestFile {
                        filename: (*path).into(),
                        previous_filename: None,
                        status: "modified".into(),
                        additions: 0,
                        deletions: 0,
                        patch: None,
                        sha: None,
                    },
                    DiffOptions::default(),
                )
            })
            .collect();
        let plan = SmartFilter {
            head_sha: "old".into(),
            base_sha: None,
            generated_at: 0,
            buckets: vec![SmartBucket {
                order: 1,
                label: "Format".into(),
                description: "Read the format".into(),
                suggest_viewed: false,
                items: vec![
                    SmartReviewItem {
                        label: "Format".into(),
                        description: "Read".into(),
                        files: vec![paths[0].into()],
                    },
                    SmartReviewItem {
                        label: "Fixtures".into(),
                        description: "Check together".into(),
                        files: vec![paths[1].into(), paths[2].into()],
                    },
                ],
            }],
        };
        let mut review = FileReviewState::default();
        // Navigation advances without marking the current stop and visits grouped files once.
        let mut navigation = ReviewWalkthrough::default();
        navigation.refresh(Some(&plan), &files, &[true; 4], &review);
        let first = WalkthroughItemId { bucket: 0, item: 0 };
        assert_eq!(navigation.next(0), Some(first));
        navigation.current_item = Some(first);
        assert_eq!(
            navigation.next(0),
            Some(WalkthroughItemId { bucket: 0, item: 1 })
        );

        review.set_viewed(&files[0], true);
        review.set_viewed(&files[1], true);
        let mut walkthrough = ReviewWalkthrough::default();
        walkthrough.refresh(Some(&plan), &files, &[false, false, true, false], &review);
        let group = WalkthroughItemId { bucket: 0, item: 1 };
        assert_eq!(walkthrough.next(0), Some(group));
        assert_eq!(walkthrough.item(group).unwrap().files, vec![1, 2]);
        assert_eq!(walkthrough.buckets[0].remaining, 1);
        assert_eq!(walkthrough.buckets[1].order, 0);
        assert_eq!(walkthrough.buckets[1].files, vec![3]);
        review.set_viewed(&files[2], true);
        walkthrough.refresh(Some(&plan), &files, &[false, false, true, false], &review);
        assert_eq!(walkthrough.next(0), None);
        assert_eq!(walkthrough.buckets[0].remaining, 0);
    }
}
