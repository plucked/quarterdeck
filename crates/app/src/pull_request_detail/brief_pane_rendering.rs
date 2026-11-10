use gpui::{AnyElement, Context, div, list, prelude::*};

use super::brief_row::BriefRow;
use super::pull_request_detail::PullRequestDetail;
use crate::brief_view::{
    BriefState, BriefWidgetContext, render_brief_bar, render_brief_column, render_brief_header,
    render_brief_page_row, render_brief_status, render_column_width_probe, render_page_tabs,
};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};

impl PullRequestDetail {
    /// The brief in place of the diff: a written brief's version line (with
    /// Report), the report panel while open, and the page tabs (several pages
    /// only), staying in view, over its rows as a virtualized list with a
    /// scrollbar.
    pub(super) fn render_brief_area(&self, cx: &mut Context<Self>) -> AnyElement {
        // A written brief's version, the report panel, and its page tabs when it has several.
        let bar = match self.current_brief() {
            Some(BriefState::Written(brief)) => {
                let handlers = self.brief().handlers();
                let tabs = (brief.pages.len() > 1)
                    .then(|| render_page_tabs(&brief.pages, self.current_brief_page(), handlers));
                Some(render_brief_bar(
                    render_brief_header(
                        &brief.record,
                        self.shown_head_sha(),
                        self.brief().report(),
                        handlers,
                    )
                    .children(tabs)
                    .into_any_element(),
                ))
            }
            _ => None,
        };

        // Measures the column, so diagrams are laid out for the width the reader gives it.
        let detail = cx.entity().downgrade();
        let probe = render_column_width_probe(self.brief().column_width(), move |width, cx| {
            detail
                .update(cx, |detail, cx| detail.set_brief_column_width(width, cx))
                .ok();
        });
        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .children(bar)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(probe)
                    .child(
                        list(
                            self.brief().list().clone(),
                            cx.processor(|detail, index, _window, _cx| {
                                detail.render_brief_row(index)
                            }),
                        )
                        .size_full(),
                    )
                    .child(VerticalScrollbar::new(
                        "brief-scrollbar",
                        ScrollbarTarget::List(self.brief().list().clone()),
                    )),
            )
            .into_any_element()
    }

    /// Row `index` of the brief, in its column.
    fn render_brief_row(&self, index: usize) -> AnyElement {
        let Some(row) = self.brief().row(index) else {
            return div().into_any_element();
        };
        let handlers = self.brief().handlers();
        let files = self.loaded_files();
        let widget_context = BriefWidgetContext {
            diffs: &files,
            reader: self.current_familiarity(),
            handlers,
        };
        let content = match (row, self.current_brief()) {
            (BriefRow::Status, state) => render_brief_status(
                state,
                !self.loaded_files().is_empty(),
                self.current_familiarity(),
                handlers,
            ),
            (BriefRow::Page(page_row), Some(BriefState::Written(brief))) => {
                render_brief_page_row(brief, self.current_brief_page(), page_row, &widget_context)
            }
            (BriefRow::Page(_), _) => div().into_any_element(),
        };
        render_brief_column(index == 0, content)
    }
}
