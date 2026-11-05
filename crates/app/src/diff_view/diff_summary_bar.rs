use std::rc::Rc;

use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*};

use super::DiffSummary;
use crate::theme;

/// What a summary bar link does when clicked.
type LinkHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// The quiet line above the changed files: totals and viewed progress, with
/// links to collapse or expand every file. How diffs look is chosen in the
/// settings window, not here, to keep the main view uncluttered.
#[derive(IntoElement)]
pub struct DiffSummaryBar {
    summary: DiffSummary,
    on_collapse_all: LinkHandler,
    on_expand_all: LinkHandler,
}

impl DiffSummaryBar {
    pub fn new(summary: DiffSummary) -> Self {
        Self {
            summary,
            on_collapse_all: Rc::new(|_, _| {}),
            on_expand_all: Rc::new(|_, _| {}),
        }
    }

    pub fn on_collapse_all(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_collapse_all = Rc::new(handler);
        self
    }

    pub fn on_expand_all(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_expand_all = Rc::new(handler);
        self
    }
}

impl RenderOnce for DiffSummaryBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let summary = self.summary;
        let collapse_all = self.on_collapse_all;
        let expand_all = self.on_expand_all;
        div()
            .flex()
            .items_center()
            .gap_3()
            .text_sm()
            .text_color(theme::muted())
            .child(div().flex_1().min_w_0().child(format!(
                "{} files · +{} −{} · {}/{} viewed",
                summary.file_count,
                summary.additions,
                summary.deletions,
                summary.viewed_count,
                summary.file_count
            )))
            .child(render_link(
                "collapse-all",
                "Collapse all",
                move |window, cx| {
                    collapse_all(window, cx);
                },
            ))
            .child(render_link(
                "expand-all",
                "Expand all",
                move |window, cx| {
                    expand_all(window, cx);
                },
            ))
    }
}

/// A small text link.
fn render_link(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_none()
        .cursor_pointer()
        .hover(|style| style.text_color(theme::text()))
        .on_click(move |_, window, cx| on_click(window, cx))
        .child(label)
}
