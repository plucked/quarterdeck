use std::rc::Rc;

use gpui::Context;

use super::pull_request_detail::PullRequestDetail;
use crate::brief_view::{BriefHandlers, BriefReport, ReportHost, view_action, view_handler};

/// The report panel's text field while empty.
const REPORT_PLACEHOLDER: &str = "What is wrong or weird: a part that misleads, is missing, is too long, a widget that looks off…";

/// What the brief's links and buttons do in the detail: an anchor opens the
/// diff there, the buttons run or cancel the agent, tabs switch pages. The
/// handlers hold a weak handle, so a closed view ignores them.
pub(super) fn detail_brief_handlers(cx: &mut Context<PullRequestDetail>) -> BriefHandlers {
    let detail = cx.entity().downgrade();
    BriefHandlers {
        show_anchor: Rc::new({
            let detail = detail.clone();
            move |anchor, _, cx| {
                detail
                    .update(cx, |detail, cx| detail.show_anchor_in_diff(anchor, cx))
                    .ok();
            }
        }),
        write: view_action(&detail, |detail, _, cx| detail.write_brief(cx)),
        cancel: view_action(&detail, |detail, _, cx| detail.cancel_brief(cx)),
        show_page: view_handler(&detail, |detail, page, _, cx| {
            detail.show_brief_page(page, cx);
        }),
        choose_reader: view_handler(&detail, |detail, familiarity, _, cx| {
            detail.choose_familiarity(familiarity, cx);
        }),
        open_report: view_action(&detail, |detail, window, cx| {
            detail.open_report(REPORT_PLACEHOLDER, window, cx);
        }),
        save_report: view_action(&detail, |detail, _, cx| detail.save_shown_brief_report(cx)),
        close_report: view_action(&detail, |detail, _, cx| detail.close_report(cx)),
        copy_report_path: view_action(&detail, |detail, _, cx| detail.copy_report_path(cx)),
    }
}

/// The report panel lays itself out in the bar above the brief's rows, so
/// the detail only hands out the pane's report.
impl ReportHost for PullRequestDetail {
    fn report_mut(&mut self) -> &mut BriefReport {
        self.brief_mut().report_mut()
    }
}
