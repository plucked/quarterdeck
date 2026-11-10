//! `SectionLabel`: the small uppercase subtle label heading a panel's
//! section, a card or a group of options.

use gpui::{
    App, FontWeight, IntoElement, RenderOnce, SharedString, Styled, Window, div, prelude::*, px,
};

use crate::theme;

/// Text size of a compact label, in panels where every pixel of height
/// counts (the Checks and Review tabs, the filter chip rows).
const COMPACT_TEXT_SIZE: f32 = 10.;

/// How large a label's text is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelSize {
    /// The interface's extra-small text (`text_xs`).
    Small,
    /// [`COMPACT_TEXT_SIZE`], for dense side panels.
    Compact,
}

/// What happens to a label too long for its line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LabelOverflow {
    /// It wraps.
    Wrap,
    /// It stays on one line, cut with an ellipsis.
    Truncate,
}

/// A small label in uppercase, in the subtle text color, heading the part
/// below it. The text is given in any case and drawn in uppercase.
#[derive(IntoElement)]
pub struct SectionLabel {
    text: SharedString,
    size: LabelSize,
    weight: FontWeight,
    overflow: LabelOverflow,
}

impl SectionLabel {
    /// A semibold label of extra-small text reading `text` in uppercase.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            size: LabelSize::Small,
            weight: FontWeight::SEMIBOLD,
            overflow: LabelOverflow::Wrap,
        }
    }

    /// Draws it at the compact 10 px size, for dense side panels.
    #[must_use]
    pub fn compact(mut self) -> Self {
        self.size = LabelSize::Compact;
        self
    }

    /// Draws it in `weight` instead of semibold.
    #[must_use]
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Keeps it on one line, cut with an ellipsis when it does not fit
    /// (it may then shrink below its text's width in a flex row).
    #[must_use]
    pub fn truncate(mut self) -> Self {
        self.overflow = LabelOverflow::Truncate;
        self
    }
}

impl RenderOnce for SectionLabel {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let label = match self.size {
            LabelSize::Small => div().text_xs(),
            LabelSize::Compact => div().text_size(px(COMPACT_TEXT_SIZE)),
        };
        let label = match self.overflow {
            LabelOverflow::Wrap => label,
            LabelOverflow::Truncate => label.min_w_0().truncate(),
        };
        label
            .font_weight(self.weight)
            .text_color(theme::subtle())
            .child(self.text.to_uppercase())
    }
}
