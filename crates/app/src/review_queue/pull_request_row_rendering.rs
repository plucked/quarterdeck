use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px};
use qd_github::{PullRequest, PullRequestBranches};

use super::ReviewQueue;
use super::sidebar_rendering::{render_meta, row_frame};
use crate::avatar_images::Avatar;
use crate::display_text::relative_age;
use crate::review_verdict_badge::ReviewVerdictBadge;
use crate::theme;

/// The open / draft ring before a pull request's title.
const STATE_RING_SIZE: f32 = 10.;
/// Text size of the mark on a pull request with a stored brief.
const BRIEF_MARK_SIZE: f32 = 11.;
/// The author's avatar in a pull request row's second line.
const ROW_AVATAR_SIZE: f32 = 14.;
/// Element id of a row's review verdict badge (unique within its row).
pub(super) const VERDICT_BADGE_ID: &str = "review-verdict";
/// Space above the state ring, so it sits level with the title's first line.
const STATE_RING_TOP: f32 = 4.;
/// Text size of the Draft pill.
const DRAFT_PILL_TEXT_SIZE: f32 = 10.;

/// Whether a pull request is ready for review or still a draft, as its
/// sidebar row shows it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PullRequestState {
    Open,
    Draft,
}

impl PullRequestState {
    fn of(pull_request: &PullRequest) -> Self {
        if pull_request.draft {
            Self::Draft
        } else {
            Self::Open
        }
    }

    /// The ring before the title: green when open, grey for a draft.
    fn ring_color(self) -> gpui::Rgba {
        match self {
            Self::Open => theme::success(),
            Self::Draft => theme::subtle(),
        }
    }

    /// A draft's title reads quieter.
    fn title_color(self) -> gpui::Rgba {
        match self {
            Self::Open => theme::text(),
            Self::Draft => theme::muted(),
        }
    }
}

/// One standalone pull request: a state ring (open or draft), its title on
/// up to two lines, then `#number · author · age`, the viewer's review
/// verdict, a Draft pill, and a mark when it has a stored brief, then `base ← head` once its branches are
/// known; clicking selects it.
pub(super) fn render_pull_request_row(
    pull_request: &PullRequest,
    list_index: usize,
    is_selected: bool,
    has_brief: bool,
    now: i64,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let state = PullRequestState::of(pull_request);
    row_frame(("pull-request", list_index), is_selected)
        .flex()
        .gap_2p5()
        .on_click(cx.listener(move |queue, _, _, cx| queue.select(list_index, cx)))
        .child(
            div().flex_none().pt(px(STATE_RING_TOP)).child(
                div()
                    .size(px(STATE_RING_SIZE))
                    .rounded_full()
                    .border_2()
                    .border_color(state.ring_color()),
            ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(state.title_color())
                        .line_clamp(2)
                        .child(pull_request.title.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .child(
                            div()
                                .flex_none()
                                .child(render_meta(format!("#{} ·", pull_request.number))),
                        )
                        .child(Avatar::new(
                            pull_request.author.avatar_url.clone(),
                            px(ROW_AVATAR_SIZE),
                        ))
                        .child(render_meta(format!(
                            "{} · {}",
                            pull_request.author.login,
                            relative_age(&pull_request.updated_at, now)
                        )))
                        .children(
                            pull_request
                                .viewer_review
                                .map(|review| ReviewVerdictBadge::new(VERDICT_BADGE_ID, review)),
                        )
                        .when(state == PullRequestState::Draft, |meta| {
                            meta.child(render_draft_pill())
                        })
                        .when(has_brief, |meta| meta.child(render_brief_mark())),
                )
                .children(pull_request.branches.as_ref().map(render_branches)),
        )
        .into_any_element()
}

/// A pull request's branches as `base ← head`, in the code font: the
/// branch it goes into, then the one it comes from. Each name truncates on
/// its own, so a long head branch leaves the base readable.
fn render_branches(branches: &PullRequestBranches) -> impl IntoElement {
    let branch_name = |name: &str| {
        div()
            .min_w_0()
            .truncate()
            .child(SharedString::from(name.to_owned()))
    };
    div()
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        .font_family(theme::MONO_FONT)
        .text_xs()
        .text_color(theme::subtle())
        .child(branch_name(&branches.base_ref))
        .child(div().flex_none().child("←"))
        .child(branch_name(&branches.head_ref))
}

/// The mark of a pull request with a stored brief: a small star in the
/// accent color.
pub(super) fn render_brief_mark() -> impl IntoElement {
    div()
        .flex_none()
        .text_size(px(BRIEF_MARK_SIZE))
        .text_color(theme::accent())
        .child("✦")
}

/// The pill marking a draft.
fn render_draft_pill() -> impl IntoElement {
    div()
        .flex_none()
        .px_1p5()
        .rounded_full()
        .border_1()
        .border_color(theme::border_strong())
        .text_size(px(DRAFT_PILL_TEXT_SIZE))
        .text_color(theme::muted())
        .child("Draft")
}
