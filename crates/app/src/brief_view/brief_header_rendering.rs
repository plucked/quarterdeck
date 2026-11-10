use gpui::{Div, div, prelude::*};
use qd_studio::BriefRecord;

use super::brief_handlers::BriefHandlers;
use super::report_rendering::render_report_panel;
use super::report_state::ReportState;
use super::version_rendering::render_brief_version;

/// What sits above a written brief, pinned while it scrolls: the version
/// line (with Report), then the report panel while it is open. The view
/// showing the brief may add more under it (the page tabs).
pub fn render_brief_header(
    record: &BriefRecord,
    shown_head: Option<&str>,
    report: &ReportState,
    handlers: &BriefHandlers,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(render_brief_version(record, shown_head, report, handlers))
        .children(render_report_panel(report, handlers))
}
