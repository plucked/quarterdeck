//! `ReviewVerdictBadge`: a small round mark saying how the viewer last
//! reviewed a pull request.

use gpui::{App, FontWeight, IntoElement, RenderOnce, Rgba, Window, div, prelude::*, px};
use qd_cache::QueueTab;
use qd_github::ViewerReview;

use crate::controls::{Pill, PillStyle, TextTooltip};
use crate::theme;

/// Diameter of the badge.
const BADGE_SIZE: f32 = 14.;
/// Size of the glyph inside it.
const GLYPH_SIZE: f32 = 9.;

/// The viewer's latest review as a 14 px circle: a check on the success
/// color (approved), a cross on the danger color (changes requested), an
/// ellipsis on a quiet ring (commented) or a dash on a faint ring
/// (dismissed); hovering says it in words.
#[derive(IntoElement)]
pub struct ReviewVerdictBadge {
    listed_in: Option<QueueTab>,
    review: ViewerReview,
}

/// How a badge is drawn.
struct BadgeLook {
    glyph: &'static str,
    /// The fill; `None` draws a ring instead.
    fill: Option<Rgba>,
    glyph_color: Rgba,
    words: &'static str,
}

impl ReviewVerdictBadge {
    /// A previous verdict in the queue, labeled "Review again" on a pending request.
    /// An unlisted stack layer shows its last verdict.
    pub fn new(listed_in: Option<QueueTab>, review: ViewerReview) -> Self {
        Self { listed_in, review }
    }
}

/// How the badge of `review` looks and what its tooltip says.
fn look_of(review: ViewerReview) -> BadgeLook {
    match review {
        ViewerReview::Approved => BadgeLook {
            glyph: "✓",
            fill: Some(theme::success()),
            glyph_color: theme::on_label(),
            words: "You approved",
        },
        ViewerReview::ChangesRequested => BadgeLook {
            glyph: "✕",
            fill: Some(theme::danger()),
            glyph_color: theme::on_label(),
            words: "You requested changes",
        },
        ViewerReview::Commented => BadgeLook {
            glyph: "…",
            fill: None,
            glyph_color: theme::muted(),
            words: "You commented",
        },
        ViewerReview::Dismissed => BadgeLook {
            glyph: "–",
            fill: None,
            glyph_color: theme::subtle(),
            words: "Your review was dismissed",
        },
    }
}

impl RenderOnce for ReviewVerdictBadge {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let look = look_of(self.review);
        let badge = div()
            .id("review-verdict")
            .flex_none()
            .tooltip(TextTooltip::builder(look.words));
        if self.listed_in == Some(QueueTab::ReviewRequested) {
            return badge.child(Pill::new("Review again", PillStyle::Outlined).compact());
        }
        badge
            .size(px(BADGE_SIZE))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .map(|badge| match look.fill {
                Some(fill) => badge.bg(fill),
                None => badge.border_1().border_color(theme::border_strong()),
            })
            .text_size(px(GLYPH_SIZE))
            .font_weight(FontWeight::BOLD)
            .text_color(look.glyph_color)
            .child(look.glyph)
    }
}
