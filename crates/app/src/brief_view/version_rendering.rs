use gpui::{AnyElement, IntoElement, div, prelude::*, px};
use qd_studio::BriefRecord;

use super::brief_handlers::{BriefHandlers, on_click};
use super::report_state::ReportState;
use crate::button::Button;
use crate::display_date::{time_ago, unix_now};
use crate::short_sha::short_sha;
use crate::theme;

/// Width of the bar marking a brief written for an older version.
const STALE_BAR_WIDTH: f32 = 3.;

/// Which version a brief was written for, against the shown one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BriefVersion<'a> {
    /// The shown head commit.
    Current(&'a str),
    /// Another commit: the brief's, then the shown one.
    Older { written: &'a str, shown: &'a str },
    /// One of the two is not known.
    Unknown,
}

/// The line above a written brief: which commit it was written for and how
/// long ago, quiet when that is the shown head; marked, with Write again,
/// when the pull request (or stack range) has moved on since; and the
/// Report button at its right end, pressed while the report panel is open
/// (a click then closes it).
pub fn render_brief_version(
    record: &BriefRecord,
    shown_head: Option<&str>,
    report: &ReportState,
    handlers: &BriefHandlers,
) -> AnyElement {
    let is_reporting = !matches!(report, ReportState::Closed);
    let toggle_report = match report {
        ReportState::Closed => &handlers.open_report,
        ReportState::Writing(_)
        | ReportState::Failed { .. }
        | ReportState::Saving
        | ReportState::Saved(_) => &handlers.close_report,
    };
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(render_version_text(record, shown_head, handlers)),
        )
        .child(
            div().flex_none().child(
                Button::new("report-brief", "Report…")
                    .ghost()
                    .selected(is_reporting)
                    .on_click(on_click(toggle_report)),
            ),
        )
        .into_any_element()
}

/// What the version line says about the commit the brief was written for.
fn render_version_text(
    record: &BriefRecord,
    shown_head: Option<&str>,
    handlers: &BriefHandlers,
) -> AnyElement {
    let age = time_ago(
        i64::try_from(record.written_at).unwrap_or(i64::MAX),
        unix_now(),
    );
    let version = match (record.head_sha.as_deref(), shown_head) {
        (Some(written), Some(shown)) if written == shown => BriefVersion::Current(written),
        (Some(written), Some(shown)) => BriefVersion::Older { written, shown },
        _ => BriefVersion::Unknown,
    };
    match version {
        BriefVersion::Current(head) => div()
            .text_xs()
            .text_color(theme::subtle())
            .child(format!(
                "Written {age} for the head commit {}",
                short_sha(head)
            ))
            .into_any_element(),
        BriefVersion::Unknown => div()
            .text_xs()
            .text_color(theme::subtle())
            .child(format!(
                "Written {age}; its commit is not known, so it may not match the version shown"
            ))
            .into_any_element(),
        BriefVersion::Older { written, shown } => div()
            .flex()
            .gap_3()
            .child(
                div()
                    .flex_none()
                    .w(px(STALE_BAR_WIDTH))
                    .rounded_sm()
                    .bg(theme::removed()),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_xs()
                    .child(format!(
                        "Written {age} for an older version (commit {}). The pull request is now at {}, so parts of the brief may no longer match the diff.",
                        short_sha(written),
                        short_sha(shown)
                    )),
            )
            .child(
                div().flex().items_center().child(
                    Button::new("rewrite-stale-brief", "Write again")
                        .on_click(on_click(&handlers.write)),
                ),
            )
            .into_any_element(),
    }
}
