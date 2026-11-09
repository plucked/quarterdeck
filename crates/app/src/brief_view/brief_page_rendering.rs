use gpui::{AnyElement, IntoElement, div, prelude::*};

use super::brief_page_row::BriefPageRow;
use super::brief_rendering::{
    render_brief_footer, render_brief_widget, render_lead, render_rejected,
};
use super::brief_widget_context::BriefWidgetContext;
use super::page_rendering::{render_page_intro, render_page_navigation};
use super::prepared_brief::PreparedBrief;

/// Row `row` of page `page` of `brief`.
pub fn render_brief_page_row(
    brief: &PreparedBrief,
    page: usize,
    row: BriefPageRow,
    context: &BriefWidgetContext,
) -> AnyElement {
    let page = page.min(brief.pages.len().saturating_sub(1));
    match row {
        BriefPageRow::Lead => render_lead(&brief.lead),
        BriefPageRow::PageIntro => match brief.pages.get(page) {
            Some(shown) => render_page_intro(shown, page, brief.pages.len()),
            None => div().into_any_element(),
        },
        BriefPageRow::Widget(index) => match brief.widgets.get(index) {
            Some(widget) => render_brief_widget(index, widget, context),
            None => div().into_any_element(),
        },
        BriefPageRow::PageNavigation => {
            render_page_navigation(&brief.pages, page, context.handlers)
        }
        BriefPageRow::Rejected => render_rejected(&brief.record.rejected),
        BriefPageRow::Footer => div()
            .pb_6()
            .child(render_brief_footer(brief, context.reader, context.handlers))
            .into_any_element(),
    }
}
