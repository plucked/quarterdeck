use gpui::{Context, ListOffset, px};

use super::pull_request_detail::PullRequestDetail;
use super::walkthrough_item_id::WalkthroughItemId;
use super::walkthrough_row::WalkthroughRow;

impl PullRequestDetail {
    /// Returns to ordinary browsing while retaining the guided step and its scroll for later.
    pub(super) fn return_to_file_diff(&mut self, cx: &mut Context<Self>) {
        self.clear_diff_selection();
        self.row_tops_width = None;
        self.focus_drawer.preview = None;
        self.focus_drawer.preview_task = None;
        self.set_detail_mode(super::detail_mode::DetailMode::Diff, cx);
    }

    /// Expands one bucket while retaining the overall reading order.
    pub(super) fn toggle_walkthrough_bucket(&mut self, bucket: usize, cx: &mut Context<Self>) {
        let Some(bucket) = self.walkthrough.buckets.get(bucket) else {
            return;
        };
        self.walkthrough.open_bucket = if self.walkthrough.open_bucket == Some(bucket.order) {
            None
        } else {
            Some(bucket.order)
        };
        self.refresh_tree_rows();
        cx.notify();
    }

    /// Shows or hides the bucket explanation without changing its files.
    pub(super) fn toggle_walkthrough_guidance(&mut self, order: u32, cx: &mut Context<Self>) {
        if !self.walkthrough.guidance.remove(&order) {
            self.walkthrough.guidance.insert(order);
        }
        self.refresh_tree_rows();
        cx.notify();
    }

    /// Reveals a grouped item’s individual files inside the virtualized list.
    pub(super) fn toggle_walkthrough_members(
        &mut self,
        id: WalkthroughItemId,
        cx: &mut Context<Self>,
    ) {
        if !self.walkthrough.expanded_items.remove(&id) {
            self.walkthrough.expanded_items.insert(id);
        }
        self.refresh_tree_rows();
        cx.notify();
    }

    /// Opens every guided stop in its own diff workspace, including single-file guidance.
    pub(super) fn select_walkthrough_item(
        &mut self,
        id: WalkthroughItemId,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.walkthrough.item(id) else {
            return;
        };
        if self.walkthrough.selected_group != Some(id) {
            self.walkthrough.group_diff = super::group_diff::GroupDiff::for_members(
                &item.files,
                self.loaded_files(),
                self.file_review(),
            );
        }
        self.focus_drawer.preview = None;
        self.focus_drawer.preview_task = None;
        self.walkthrough.selected_group = Some(id);
        self.refresh_group_diff();
        self.walkthrough.current_item = Some(id);
        self.set_detail_mode(super::detail_mode::DetailMode::GuidedReview, cx);
        cx.notify();
    }

    /// Chooses one incomplete item, skipping completed groups as a whole.
    pub(super) fn next_walkthrough_item(&mut self, bucket: usize, cx: &mut Context<Self>) {
        let Some(id) = self.walkthrough.next(bucket) else {
            return;
        };
        self.reveal_walkthrough_item(id, cx);
    }

    /// Continues through the guided order across buckets, skipping completed or hidden items.
    pub(super) fn advance_walkthrough(
        &mut self,
        current: WalkthroughItemId,
        cx: &mut Context<Self>,
    ) {
        if let Some(id) = self.walkthrough.next_after(current) {
            self.reveal_walkthrough_item(id, cx);
        }
    }

    /// Opens the destination bucket and brings its selected item into view.
    fn reveal_walkthrough_item(&mut self, id: WalkthroughItemId, cx: &mut Context<Self>) {
        // Make the destination's row available before selecting and scrolling to it.
        self.walkthrough.open_bucket = Some(self.walkthrough.buckets[id.bucket].order);
        self.refresh_tree_rows();
        self.select_walkthrough_item(id, cx);
        if let Some(row) = self
            .walkthrough
            .rows
            .iter()
            .position(|row| matches!(row,WalkthroughRow::Item(other) if *other == id))
        {
            self.walkthrough.list.scroll_to(ListOffset {
                item_ix: row,
                offset_in_item: px(0.),
            });
        }
        cx.notify();
    }

    /// Marks a stop and immediately continues; unmarking leaves the reader in place.
    pub(super) fn mark_walkthrough_viewed_and_advance(
        &mut self,
        id: WalkthroughItemId,
        cx: &mut Context<Self>,
    ) {
        // Capture the direction before the viewed update rebuilds the navigation counts.
        let Some(item) = self.walkthrough.item(id) else {
            return;
        };
        let was_viewed = item.viewed == item.files.len();
        self.toggle_walkthrough_viewed(id, cx);
        // Keep the completed group available when no unfinished destination remains.
        if !was_viewed {
            self.advance_walkthrough(id, cx);
        }
    }

    /// Explicitly acknowledges all current members, including those hidden by filters.
    pub(super) fn toggle_walkthrough_viewed(
        &mut self,
        id: WalkthroughItemId,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.walkthrough.item(id) else {
            return;
        };
        let viewed = item.viewed < item.files.len();
        let files = item.files.clone();
        self.set_files_viewed(&files, viewed, cx);
    }

    /// Marks the full saved membership of a bucket, including currently hidden files.
    pub(super) fn mark_walkthrough_bucket(&mut self, bucket: usize, cx: &mut Context<Self>) {
        let Some(bucket) = self.walkthrough.buckets.get(bucket) else {
            return;
        };
        let files = bucket.files.clone();
        self.set_files_viewed(&files, true, cx);
    }

    /// Shows the unmapped current paths without restrictive filters hiding them.
    pub(super) fn show_unplanned_files(&mut self, cx: &mut Context<Self>) {
        self.file_tree.clear_chips();
        if self.file_tree.filter().hide_viewed {
            self.file_tree.toggle_hide_viewed();
        }
        self.focus_drawer
            .input
            .update(cx, |input, cx| input.set_text(String::new(), cx));
        self.file_tree.search.query.clear();
        self.walkthrough.open_bucket = Some(0);
        self.rebuild_rows(self.anchor_at_top());
        if let Some(row) = self.walkthrough.rows.iter().position(|row| matches!(row,WalkthroughRow::Bucket(index) if self.walkthrough.buckets[*index].order == 0)) {
            self.walkthrough.list.scroll_to(ListOffset {item_ix:row,offset_in_item:px(0.)});
        }
        cx.notify();
    }
}
