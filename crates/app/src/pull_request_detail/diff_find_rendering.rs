use gpui::{Context, Div, div, prelude::*};

use super::file_direction::FileDirection;
use super::pull_request_detail::PullRequestDetail;
use crate::app_commands::PreviousDiffMatch;
use crate::controls::{Button, TextTooltip};
use crate::diff_view::{CellMatches, DiffSide};
use crate::theme;

impl PullRequestDetail {
    /// Paints only this file's matches. Split context shares one navigation stop across both cells.
    pub(super) fn diff_matches_for_row(&self, row: usize) -> [CellMatches; 2] {
        let mut cells = [CellMatches::default(), CellMatches::default()];
        let Some(find) = &self.diff_find else {
            return cells;
        };
        let Some(range) = self
            .rows()
            .file_range(find.file)
            .filter(|range| range.contains(&row))
        else {
            return cells;
        };
        for (index, found) in find.matches.on_row(row - range.start) {
            let mut add = |column: usize| {
                cells[column].ranges.push(found.range.clone());
                if index == find.current {
                    cells[column].current = Some(found.range.clone());
                    cells[column].reveal = Some(find.reveal.clone());
                }
            };
            add(usize::from(found.side == DiffSide::New));
            if found.mirrored {
                add(1);
            }
        }
        cells
    }

    /// The current file's Find field, result count, navigation and dismissal.
    pub(super) fn render_diff_find(&self, cx: &Context<Self>) -> Option<Div> {
        let find = self.diff_find.as_ref()?;
        let path = self.loaded_files().get(find.file)?.filename();
        let name = path.rsplit('/').next().unwrap_or(path);
        let count = find.matches.occurrences.len();
        let label = if find.input.read(cx).text().is_empty() {
            String::new()
        } else if find.task.is_some() {
            "Searching…".to_owned()
        } else if count == 0 {
            "No matches".to_owned()
        } else {
            format!("{} of {count}", find.current + 1)
        };
        Some(
            div()
                .key_context("DiffFind")
                .on_action(cx.listener(|detail, _: &PreviousDiffMatch, window, cx| {
                    detail.move_diff_match(FileDirection::Previous, window, cx);
                }))
                .flex_none()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .border_b_1()
                .border_color(theme::border())
                .bg(theme::panel())
                .child(
                    div()
                        .min_w(gpui::px(112.))
                        .flex_1()
                        .child(find.input.clone()),
                )
                .child(
                    div()
                        .id("diff-find-filename")
                        .max_w_1_4()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(theme::muted())
                        .tooltip(TextTooltip::builder(path.to_owned()))
                        .child(name.to_owned()),
                )
                .child(
                    div()
                        .flex_none()
                        .px_1()
                        .text_xs()
                        .text_color(theme::muted())
                        .child(label),
                )
                .child(
                    Button::new("previous-diff-match", "Previous")
                        .ghost()
                        .disabled(count == 0)
                        .on_click(cx.listener(|detail, _, window, cx| {
                            detail.move_diff_match(FileDirection::Previous, window, cx);
                        })),
                )
                .child(
                    Button::new("next-diff-match", "Next")
                        .ghost()
                        .disabled(count == 0)
                        .on_click(cx.listener(|detail, _, window, cx| {
                            detail.move_diff_match(FileDirection::Next, window, cx);
                        })),
                )
                .child(Button::new("close-diff-find", "Done").ghost().on_click(
                    cx.listener(|detail, _, window, cx| detail.close_diff_find(window, cx)),
                )),
        )
    }
}
