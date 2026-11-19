use gpui::{AppContext as _, Context, ListOffset, Window, px};

use super::detail_mode::DetailMode;
use super::diff_find::DiffFind;
use super::diff_find_matches::DiffFindMatches;
use super::file_direction::FileDirection;
use super::pull_request_detail::PullRequestDetail;
use crate::diff_view::DiffMatchReveal;
use crate::text_area::{TextArea, TextAreaEvent, TextAreaSelectAll};

impl PullRequestDetail {
    /// Opens Find for the current file in either diff workspace and focuses the query.
    pub fn open_diff_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus_drawer.preview.is_some()
            || !(self.brief().mode == DetailMode::Diff || self.guided_diff_active())
        {
            return;
        }
        // The ordinary list starts with a summary; its first file begins on row one.
        let Some(file) = self.current_file().or_else(|| self.rows().file_at_row(1)) else {
            return;
        };
        let Some(loaded) = self.loaded_files().get(file) else {
            return;
        };
        let collapsed = if self.guided_diff_active() {
            !self.walkthrough.group_diff.expanded.contains(&file)
        } else {
            self.file_review.state_of(loaded).is_collapsed
        };
        if collapsed {
            if self.guided_diff_active() {
                self.toggle_group_file(file, cx);
            } else {
                self.toggle_file_collapsed(file, cx);
            }
        }

        if let Some(find) = &self.diff_find
            && find.file == file
        {
            find.input.read(cx).focus(window);
            window.dispatch_action(Box::new(TextAreaSelectAll), cx);
            return;
        }

        // Text changes search only this file. Rebuilt rows invalidate positions before the next paint.
        let input = cx.new(|cx| TextArea::single_line("Find in diff…", cx));
        let on_input =
            cx.subscribe_in(&input, window, |detail, _, event, window, cx| match event {
                TextAreaEvent::Changed => detail.search_diff(true, cx),
                TextAreaEvent::LayoutChanged | TextAreaEvent::FormattingChanged => {}
                TextAreaEvent::Submit => detail.move_diff_match(FileDirection::Next, window, cx),
                TextAreaEvent::Cancel => detail.close_diff_find(window, cx),
            });
        let on_rows = cx.observe_self(|detail, cx| {
            if detail
                .diff_find
                .as_ref()
                .is_some_and(|find| find.rows_changed)
            {
                detail.search_diff(false, cx);
            }
        });
        let detail = cx.weak_entity();
        let reveal = DiffMatchReveal::new(move |offset, cx| {
            let detail = detail.clone();
            cx.defer(move |cx| {
                detail
                    .update(cx, |detail, cx| {
                        if detail
                            .diff_find
                            .as_ref()
                            .is_some_and(|find| find.file == file)
                        {
                            detail.horizontal_offsets.insert(file, offset);
                            cx.notify();
                        }
                    })
                    .ok();
            });
        });
        input.read(cx).focus(window);
        self.diff_find = Some(DiffFind {
            file,
            input,
            matches: DiffFindMatches::default(),
            current: 0,
            reveal,
            task: None,
            rows_changed: false,
            _subscriptions: [on_input, on_rows],
        });
        cx.notify();
    }

    /// Drops positions before changed rows can draw or navigate to a stale match.
    pub(super) fn invalidate_diff_find(&mut self) {
        if let Some(find) = &mut self.diff_find {
            find.task = None;
            find.matches = DiffFindMatches::default();
            find.rows_changed = true;
        }
    }

    /// Searches a snapshot off the window thread. Dropping the task cancels an obsolete query.
    fn search_diff(&mut self, reveal_first: bool, cx: &mut Context<Self>) {
        let Some(find) = &mut self.diff_find else {
            return;
        };
        find.rows_changed = false;
        find.task = None;
        find.matches = DiffFindMatches::default();
        if reveal_first {
            find.current = 0;
        }
        let file = find.file;
        let query = find.input.read(cx).text().to_owned();

        // A hidden or unloaded file no longer has a diff in this workspace.
        let Some(range) = self
            .rows()
            .file_range(file)
            .filter(|range| !range.is_empty())
        else {
            self.diff_find = None;
            cx.notify();
            return;
        };
        if query.is_empty() {
            cx.notify();
            return;
        }
        let diff = self.loaded_files()[file].diff.clone();
        let rows = self.rows().rows_in(range).to_vec();
        let task = cx.spawn(async move |detail, cx| {
            let matches = cx
                .background_executor()
                .spawn(async move { DiffFindMatches::search(&diff, &rows, &query) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    let Some(find) = &mut detail.diff_find else {
                        return;
                    };
                    find.current = find
                        .current
                        .min(matches.occurrences.len().saturating_sub(1));
                    find.matches = matches;
                    find.task = None;
                    if reveal_first {
                        detail.reveal_diff_match();
                    }
                    cx.notify();
                })
                .ok();
        });
        if let Some(find) = &mut self.diff_find {
            find.task = Some(task);
        }
        cx.notify();
    }

    /// Chooses the next or previous occurrence, wrapping at either end.
    pub(super) fn move_diff_match(
        &mut self,
        direction: FileDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(find) = &mut self.diff_find else {
            return;
        };
        find.input.read(cx).focus(window);
        let count = find.matches.occurrences.len();
        if count == 0 {
            return;
        }
        find.current = match direction {
            FileDirection::Next => (find.current + 1) % count,
            FileDirection::Previous => find.current.checked_sub(1).unwrap_or(count - 1),
        };
        self.reveal_diff_match();
        cx.notify();
    }

    /// Brings the matching row into layout, where the code cell reveals the exact occurrence.
    fn reveal_diff_match(&mut self) {
        let Some(find) = &self.diff_find else { return };
        let Some(found) = find.matches.occurrences.get(find.current) else {
            return;
        };
        let Some(start) = self.rows().file_start(find.file) else {
            return;
        };
        let row = start + found.row;
        find.reveal.request();
        let list = self.list();
        if !list
            .bounds_for_item(row)
            .is_some_and(|bounds| bounds.intersects(&list.viewport_bounds()))
        {
            list.scroll_to(ListOffset {
                item_ix: row,
                offset_in_item: px(0.),
            });
        }
    }

    /// Dismisses the field and returns keyboard focus to the diff.
    pub(super) fn close_diff_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.diff_find = None;
        window.focus(self.focus());
        cx.notify();
    }
}
