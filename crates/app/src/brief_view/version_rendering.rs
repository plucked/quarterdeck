use gpui::{AnyElement, IntoElement, div, prelude::*, px};
use qd_studio::BriefRecord;

use super::brief_handlers::{BriefHandlers, on_click};
use super::report_state::ReportState;
use crate::controls::{Button, NumberFlow};
use crate::display_text::{short_sha, time_ago, unix_now};
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
    /// The head is unchanged, but a combined range now compares another base.
    DifferentBase(&'a str),
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
    shown_base: Option<&str>,
    is_writing: bool,
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
        .child(div().flex_1().min_w_0().child(render_version_text(
            record, shown_head, shown_base, is_writing, handlers,
        )))
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
    shown_base: Option<&str>,
    is_writing: bool,
    handlers: &BriefHandlers,
) -> AnyElement {
    let age = time_ago(
        i64::try_from(record.written_at).unwrap_or(i64::MAX),
        unix_now(),
    );
    let version = brief_version(
        record.head_sha.as_deref(),
        record.mcp.as_ref().and_then(|mcp| mcp.base_sha.as_deref()),
        record.mcp.is_some(),
        shown_head,
        shown_base,
    );
    match version {
        BriefVersion::Current(head) => div()
            .text_xs()
            .text_color(theme::subtle())
            .child(NumberFlow::label(format!(
                "Written {age} for the head commit {}",
                short_sha(head)
            )))
            .into_any_element(),
        BriefVersion::Unknown => div()
            .text_xs()
            .text_color(theme::subtle())
            .child(NumberFlow::label(format!(
                "Written {age}; its commit is not known, so it may not match the version shown"
            )))
            .into_any_element(),
        BriefVersion::Older { written, shown } => render_stale_version(
            format!(
                "Written {age} for an older version (commit {}). The pull request is now at {}, so parts of the brief may no longer match the diff.",
                short_sha(written),
                short_sha(shown)
            ),
            is_writing,
            handlers,
        ),
        BriefVersion::DifferentBase(head) => render_stale_version(
            format!(
                "Written {age} for head {} with a different comparison base. Parts of the brief may no longer match the diff.",
                short_sha(head)
            ),
            is_writing,
            handlers,
        ),
    }
}

/// Legacy briefs know only their head; MCP briefs also capture the comparison base.
fn brief_version<'a>(
    written_head: Option<&'a str>,
    written_base: Option<&str>,
    knows_base: bool,
    shown_head: Option<&'a str>,
    shown_base: Option<&str>,
) -> BriefVersion<'a> {
    match (written_head, shown_head) {
        (Some(written), Some(shown)) if written != shown => BriefVersion::Older { written, shown },
        (Some(written), Some(_)) if knows_base && written_base != shown_base => {
            BriefVersion::DifferentBase(written)
        }
        (Some(written), Some(_)) => BriefVersion::Current(written),
        _ => BriefVersion::Unknown,
    }
}

/// Keeps the established stale-version treatment for either side of the comparison.
fn render_stale_version(message: String, is_writing: bool, handlers: &BriefHandlers) -> AnyElement {
    div()
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
                .child(NumberFlow::label(message)),
        )
        .child(
            div().flex().items_center().child(
                Button::new("rewrite-stale-brief", "Write again")
                    .disabled(is_writing)
                    .on_click(on_click(&handlers.write)),
            ),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::{BriefVersion, brief_version};

    #[test]
    fn base_only_changes_are_stale_when_the_saved_brief_captured_a_base() {
        assert_eq!(
            brief_version(Some("head"), Some("old"), true, Some("head"), Some("new")),
            BriefVersion::DifferentBase("head")
        );
        assert_eq!(
            brief_version(Some("head"), Some("base"), true, Some("head"), Some("base")),
            BriefVersion::Current("head")
        );
        assert_eq!(
            brief_version(Some("head"), None, false, Some("head"), Some("base")),
            BriefVersion::Current("head")
        );
    }
}
