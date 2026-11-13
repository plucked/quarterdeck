use std::rc::Rc;

use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*};

use super::DiffSummary;
use crate::controls::{Button, NumberFlow};
use crate::theme;

/// What a summary bar link does when clicked.
type LinkHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// The quiet line above the changed files: totals, with links to collapse
/// or expand every file. Viewed progress shows in the side panel instead. How diffs look is chosen in the
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
            .text_xs()
            .text_color(theme::muted())
            .child(NumberFlow::label(format!("{} files", summary.file_count)))
            .child(
                div()
                    .text_color(theme::added())
                    .child(NumberFlow::label(format!("+{}", summary.additions))),
            )
            .child(
                div()
                    .text_color(theme::removed())
                    .child(NumberFlow::label(format!("−{}", summary.deletions))),
            )
            .child(div().flex_1())
            .child(
                Button::new("collapse-all", "Collapse all")
                    .ghost()
                    .on_click(move |_, window, cx| collapse_all(window, cx)),
            )
            .child(
                Button::new("expand-all", "Expand all")
                    .ghost()
                    .on_click(move |_, window, cx| expand_all(window, cx)),
            )
    }
}
