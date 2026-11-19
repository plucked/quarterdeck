use gpui::{
    AnyElement, Context, HighlightStyle, IntoElement, StyledText, div, prelude::*, px, uniform_list,
};

use super::file_tree_rendering::render_file_icon;
use super::navigation_file_target::NavigationFileTarget;
use super::pull_request_detail::PullRequestDetail;
use super::search_preview::SearchPreview;
use super::search_scope::SearchScope;
use crate::controls::{Button, CheckState, Checkbox, NumberFlow, TextTooltip};
use crate::controls::{FoldChevron, FoldState};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

impl PullRequestDetail {
    /// Ranked search results remain virtualized; content snippets show their real source lines.
    pub(super) fn render_search_results(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.focus_drawer.search_files.is_empty() {
            let message = if self.focus_drawer.content_task.is_some() {
                "Loading file contents…"
            } else {
                "No files match this search and the active filters."
            };
            return div()
                .flex_1()
                .p_3()
                .text_xs()
                .text_color(theme::muted())
                .child(message)
                .into_any_element();
        }
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                uniform_list(
                    "file-search-results",
                    self.focus_drawer.search_files.len(),
                    cx.processor(|detail, range: std::ops::Range<usize>, _, cx| {
                        range
                            .filter_map(|row| detail.render_search_result(row, cx))
                            .collect()
                    }),
                )
                .track_scroll(self.focus_drawer.search_scroll.clone())
                .size_full(),
            )
            .child(VerticalScrollbar::new(
                "search-scrollbar",
                ScrollbarTarget::Handle(
                    self.focus_drawer
                        .search_scroll
                        .0
                        .borrow()
                        .base_handle
                        .clone(),
                ),
            ))
            .into_any_element()
    }

    /// One bounded result with file navigation and up to three source snippets.
    fn render_search_result(&self, row: usize, cx: &mut Context<Self>) -> Option<AnyElement> {
        let file = *self.focus_drawer.search_files.get(row)?;
        let loaded = self.loaded_files().get(file)?;
        let found = self.file_tree.search.matches.get(loaded.filename());
        let mut result = div()
            .id(("search-result", file))
            .h(px(
                if self.file_tree.search.scope == SearchScope::Contents {
                    132.
                } else {
                    48.
                },
            ))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.render_navigation_file(
                file,
                "search-file",
                NavigationFileTarget::FileDiff,
                cx,
            ));
        if self.file_tree.search.scope == SearchScope::Contents
            && let Some(found) = found
        {
            for (hit, matched) in found.lines.iter().enumerate() {
                let line = matched.line;
                result = result.child(
                    div()
                        .id(("content-hit", row * 3 + hit))
                        .px_3()
                        .h(px(22.))
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .cursor_pointer()
                        .hover(|style| style.bg(theme::hover()))
                        .on_click(cx.listener(move |detail, _, _, cx| {
                            detail.select_content_match(file, line, cx);
                        }))
                        .child(
                            div()
                                .flex_none()
                                .text_color(theme::subtle())
                                .child(line.to_string()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .font_family(theme::MONO_FONT)
                                .text_size(px(10.))
                                .text_color(theme::muted())
                                .child(StyledText::new(matched.text.clone()).with_highlights([(
                                    matched.matched.clone(),
                                    HighlightStyle {
                                        color: Some(theme::text().into()),
                                        background_color: Some(theme::text_selection().into()),
                                        ..HighlightStyle::default()
                                    },
                                )])),
                        ),
                );
            }
            result = result.child(
                div()
                    .px_3()
                    .text_size(px(10.))
                    .text_color(theme::subtle())
                    .child(NumberFlow::label(format!(
                        "{} matching lines{}",
                        found.line_count,
                        if loaded.file.status == "removed" {
                            " · base revision"
                        } else {
                            ""
                        }
                    ))),
            );
        }
        Some(result.into_any_element())
    }

    /// A normal file row shared by the walkthrough and search results.
    pub(super) fn render_navigation_file(
        &self,
        file: usize,
        prefix: &'static str,
        target: NavigationFileTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(loaded) = self.loaded_files().get(file) else {
            return div().into_any_element();
        };
        let viewed = self.file_review().is_viewed(loaded.filename());
        let (parent, name) = loaded
            .filename()
            .rsplit_once('/')
            .unwrap_or(("", loaded.filename()));
        div()
            .id((prefix, file))
            .w_full()
            .h(px(48.))
            .px_3()
            .flex()
            .items_center()
            .gap_1p5()
            .cursor_pointer()
            .hover(|style| style.bg(theme::hover()))
            .when(
                target == NavigationFileTarget::FileDiff && self.current_file() == Some(file),
                |row| row.bg(theme::selected()),
            )
            .tooltip(TextTooltip::builder(loaded.filename().to_owned()))
            .on_click(cx.listener(move |detail, _, _, cx| match target {
                NavigationFileTarget::FileDiff => detail.open_navigation_file(file, cx),
                NavigationFileTarget::GuidedItem(id) => {
                    detail.select_walkthrough_item(id, cx);
                    detail.open_navigation_file(file, cx);
                }
                NavigationFileTarget::GroupDiff => detail.toggle_group_file(file, cx),
            }))
            .when(target == NavigationFileTarget::GroupDiff, |row| {
                row.bg(theme::surface())
                    .border_1()
                    .border_color(theme::border())
                    .rounded_t_md()
                    .child(FoldChevron::new(
                        ("group-file-fold", file),
                        FoldState::of(!self.walkthrough.group_diff.expanded.contains(&file)),
                    ))
            })
            .child(render_file_icon(loaded.facets.icon, viewed))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .text_xs()
                    .child(
                        div()
                            .truncate()
                            .text_color(if viewed {
                                theme::subtle()
                            } else {
                                theme::text()
                            })
                            .child(name.to_owned()),
                    )
                    .child(
                        div()
                            .truncate()
                            .text_size(px(10.))
                            .text_color(theme::subtle())
                            .child(parent.to_owned()),
                    ),
            )
            .child(
                div()
                    .id(("navigation-viewed", file))
                    .flex_none()
                    .p_1()
                    .tooltip(TextTooltip::builder("Toggle file viewed"))
                    .on_click(cx.listener(move |detail, _, _, cx| {
                        cx.stop_propagation();
                        detail.toggle_file_viewed(file, cx);
                    }))
                    .child(Checkbox::new(CheckState::from_bool(viewed))),
            )
            .into_any_element()
    }

    /// Content matches open real source context; other navigation opens the diff.
    pub(super) fn open_navigation_file(&mut self, file: usize, cx: &mut Context<Self>) {
        let guided =
            self.focus_drawer.mode == qd_cache::FilesMode::Plan && self.open_guided_file(file, cx);
        if self.file_tree.search.scope == SearchScope::Contents
            && !self.file_tree.search.query.trim().is_empty()
            && let Some(loaded) = self.loaded_files().get(file)
            && let Some(hit) = self
                .file_tree
                .search
                .matches
                .get(loaded.filename())
                .and_then(|found| found.lines.first())
        {
            self.select_content_match(file, hit.line, cx);
            return;
        }
        if !guided {
            self.show_file_in_diff(file, cx);
        }
    }

    /// Loads a bounded source excerpt off the window thread, even when the hit is outside the patch.
    pub(super) fn select_content_match(&mut self, file: usize, line: u32, cx: &mut Context<Self>) {
        let Some(loaded) = self.loaded_files().get(file) else {
            return;
        };
        let Some(text) = self.focus_drawer.contents.get(loaded.filename()).cloned() else {
            return;
        };
        let changed_file = loaded.file.clone();
        self.clear_diff_selection();
        self.row_tops_width = None;
        let mode = match self.focus_drawer.mode {
            qd_cache::FilesMode::Tree => super::detail_mode::DetailMode::Diff,
            qd_cache::FilesMode::Plan => super::detail_mode::DetailMode::GuidedReview,
        };
        self.focus_drawer.preview_task =
            Some(cx.spawn(async move |detail, cx| {
                let excerpt = cx
                    .background_executor()
                    .spawn(async move {
                        let first = usize::try_from(line.saturating_sub(7)).unwrap_or(0);
                        SearchPreview {
                            file,
                            first_line: first + 1,
                            selected_line: line,
                            lines: text
                                .lines()
                                .skip(first)
                                .take(15)
                                .map(str::to_owned)
                                .collect(),
                        }
                    })
                    .await;
                detail
                    .update(cx, |detail, cx| {
                        if detail.loaded_files().get(file).is_some_and(|loaded| {
                            std::sync::Arc::ptr_eq(&loaded.file, &changed_file)
                        }) {
                            detail.diff_find = None;
                            detail.focus_drawer.preview = Some(excerpt);
                            detail.set_detail_mode(mode, cx);
                            cx.notify();
                        }
                    })
                    .ok();
            }));
        cx.notify();
    }

    /// Source context is explicitly labeled, so an unchanged match is never mistaken for a diff.
    pub(super) fn render_search_preview(
        &self,
        preview: &SearchPreview,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let file = preview.file;
        let Some(loaded) = self.loaded_files().get(file) else {
            return div().into_any_element();
        };
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .p_4()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .child(loaded.filename().to_owned()),
                    )
                    .child(Button::new("search-show-diff", "Show diff").on_click(
                        cx.listener(move |detail, _, _, cx| detail.show_file_in_diff(file, cx)),
                    )),
            )
            .child(div().text_xs().text_color(theme::muted()).child(
                if loaded.file.status == "removed" {
                    "Source context · base revision (deleted file)"
                } else {
                    "Source context · head revision"
                },
            ))
            .child(
                div()
                    .id("search-source-context")
                    .overflow_x_scroll()
                    .min_w_0()
                    .border_1()
                    .border_color(theme::border())
                    .rounded_md()
                    .py_2()
                    .children(preview.lines.iter().enumerate().map(|(offset, line)| {
                        let number = preview.first_line + offset;
                        div()
                            .flex()
                            .gap_3()
                            .px_3()
                            .py_0p5()
                            .font_family(theme::MONO_FONT)
                            .text_xs()
                            .whitespace_nowrap()
                            .when(
                                u32::try_from(number).ok() == Some(preview.selected_line),
                                |row| row.bg(theme::selected()),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .w(px(40.))
                                    .text_right()
                                    .text_color(theme::subtle())
                                    .child(number.to_string()),
                            )
                            .child(line.clone())
                    })),
            )
            .into_any_element()
    }
}
