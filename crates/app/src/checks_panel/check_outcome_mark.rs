use gpui::{
    AnimationExt as _, App, ElementId, FontWeight, IntoElement, RenderOnce, Rgba, Styled, Window,
    div, prelude::*, px,
};
use qd_github::CheckOutcome;

use crate::pulsing_status::dot_pulse;
use crate::text_tooltip::TextTooltip;
use crate::theme;

/// A glyph's size as a share of the mark's diameter.
const GLYPH_SHARE: f32 = 0.64;
/// A dot's diameter as a share of the mark's.
const DOT_SHARE: f32 = 0.36;

/// A check's outcome as a small round mark, in the language of the
/// review verdict badges: a check on success (passed), a cross on danger
/// (failed), a pulsing dot in an accent ring (running), a still dot in a
/// quiet ring (queued), an empty ring (expected: nothing reported yet), a
/// dash on a muted fill (cancelled), a dash in a ring (neutral) and a slash
/// in a faint ring (skipped); hovering says it.
#[derive(IntoElement)]
pub struct CheckOutcomeMark {
    id: ElementId,
    outcome: CheckOutcome,
    size: f32,
}

/// What sits inside a mark.
enum MarkInside {
    Glyph(&'static str, Rgba),
    Dot(Rgba, DotMotion),
    Nothing,
}

/// Whether a mark's dot breathes (something happens) or stays still.
enum DotMotion {
    Pulsing,
    Still,
}

/// A mark's circle: filled, or a ring.
enum MarkShape {
    Filled(Rgba),
    Ring(Rgba),
}

/// How a mark is drawn.
struct MarkLook {
    shape: MarkShape,
    inside: MarkInside,
    words: &'static str,
}

impl CheckOutcomeMark {
    /// The mark of `outcome`, `size` px across; `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, outcome: CheckOutcome, size: f32) -> Self {
        Self {
            id: id.into(),
            outcome,
            size,
        }
    }
}

/// How the mark of `outcome` looks and what its tooltip says.
fn look_of(outcome: CheckOutcome) -> MarkLook {
    match outcome {
        CheckOutcome::Passed => MarkLook {
            shape: MarkShape::Filled(theme::success()),
            inside: MarkInside::Glyph("✓", theme::on_label()),
            words: "Passed",
        },
        CheckOutcome::Failed => MarkLook {
            shape: MarkShape::Filled(theme::danger()),
            inside: MarkInside::Glyph("✕", theme::on_label()),
            words: "Failed",
        },
        CheckOutcome::Running => MarkLook {
            shape: MarkShape::Ring(theme::accent()),
            inside: MarkInside::Dot(theme::accent(), DotMotion::Pulsing),
            words: "Running",
        },
        CheckOutcome::Queued => MarkLook {
            shape: MarkShape::Ring(theme::border_strong()),
            inside: MarkInside::Dot(theme::subtle(), DotMotion::Still),
            words: "Queued",
        },
        CheckOutcome::Expected => MarkLook {
            shape: MarkShape::Ring(theme::border_strong()),
            inside: MarkInside::Nothing,
            words: "Expected: waiting for its status to be reported",
        },
        CheckOutcome::Cancelled => MarkLook {
            shape: MarkShape::Filled(theme::muted()),
            inside: MarkInside::Glyph("–", theme::panel()),
            words: "Cancelled",
        },
        CheckOutcome::Neutral => MarkLook {
            shape: MarkShape::Ring(theme::border_strong()),
            inside: MarkInside::Glyph("–", theme::muted()),
            words: "Neutral",
        },
        CheckOutcome::Skipped => MarkLook {
            shape: MarkShape::Ring(theme::border()),
            inside: MarkInside::Glyph("/", theme::subtle()),
            words: "Skipped",
        },
    }
}

impl RenderOnce for CheckOutcomeMark {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let look = look_of(self.outcome);
        let mark = div()
            .id(self.id.clone())
            .flex_none()
            .size(px(self.size))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .map(|mark| match look.shape {
                MarkShape::Filled(fill) => mark.bg(fill),
                MarkShape::Ring(ring) => mark.border_1().border_color(ring),
            })
            .tooltip(TextTooltip::builder(look.words));
        match look.inside {
            MarkInside::Glyph(glyph, color) => mark
                .text_size(px(self.size * GLYPH_SHARE))
                .font_weight(FontWeight::BOLD)
                .text_color(color)
                .child(glyph),
            MarkInside::Dot(color, motion) => {
                let dot = div()
                    .size(px(self.size * DOT_SHARE))
                    .rounded_full()
                    .bg(color);
                match motion {
                    DotMotion::Pulsing => {
                        mark.child(dot.with_animation(self.id, dot_pulse(), Styled::opacity))
                    }
                    DotMotion::Still => mark.child(dot),
                }
            }
            MarkInside::Nothing => mark,
        }
    }
}
