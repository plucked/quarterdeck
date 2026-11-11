//! `Pill`: a short label on a fully rounded shape, for badges, counts and
//! status (Draft, Review, Required, a repository's count, a settings tag).

use gpui::{
    AnyElement, App, FontWeight, IntoElement, RenderOnce, Rgba, SharedString, Styled, Window, div,
    prelude::*, px,
};

use crate::theme;

/// Text size of a compact pill, the marks inside a dense row (Draft,
/// Required, a comment's state).
const COMPACT_TEXT_SIZE: f32 = 10.;

/// What a pill's shape and text are colored with.
#[derive(Clone, Copy)]
pub enum PillStyle {
    /// A strong border around no fill, muted text: Draft, a required check.
    Outlined,
    /// Border and text in the given color, no fill: a comment's state, a
    /// failing required check.
    Tinted(Rgba),
    /// The surface color as fill, muted text: a count.
    Filled,
    /// The surface fill inside a faint border, muted text: a tag on a
    /// settings page, what the detail pane is busy with.
    Framed,
    /// The selection fill, accent text in medium weight: what asks for the
    /// viewer (Review) or where they are (Layer n of m).
    Accent,
}

/// How large a pill's text is and how much room it leaves around it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PillSize {
    /// [`COMPACT_TEXT_SIZE`] text, a little room at the sides.
    Compact,
    /// The interface's extra-small text (`text_xs`), a little room at the sides.
    Small,
    /// Extra-small text with more room at the sides, above and below.
    Roomy,
}

/// A short label on a fully rounded shape. It keeps its width in a flex
/// row and inherits the font weight around it unless its style or
/// [`Pill::weight`] sets one.
#[derive(IntoElement)]
pub struct Pill {
    text: SharedString,
    style: PillStyle,
    size: PillSize,
    weight: Option<FontWeight>,
    text_color: Option<Rgba>,
    mark: Option<AnyElement>,
}

impl Pill {
    /// A pill of extra-small text reading `text`, colored by `style`.
    pub fn new(text: impl Into<SharedString>, style: PillStyle) -> Self {
        Self {
            text: text.into(),
            style,
            size: PillSize::Small,
            weight: None,
            text_color: None,
            mark: None,
        }
    }

    /// The compact outlined pill marking a draft pull request, the same in
    /// the sidebar and the detail's header.
    pub fn draft() -> Self {
        Self::new("Draft", PillStyle::Outlined).compact()
    }

    /// Draws it at the compact 10 px size, for marks inside a dense row.
    #[must_use]
    pub fn compact(mut self) -> Self {
        self.size = PillSize::Compact;
        self
    }

    /// Gives it more room around its text, for a pill standing on its own
    /// in a header or on a settings page.
    #[must_use]
    pub fn roomy(mut self) -> Self {
        self.size = PillSize::Roomy;
        self
    }

    /// Draws its text in `weight` instead of what its style or the text
    /// around it sets.
    #[must_use]
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Colors the label while keeping the style's fill and border.
    #[must_use]
    pub fn text_color(mut self, color: Rgba) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Puts `mark` (a dot, a small icon) before the text.
    #[must_use]
    pub fn mark(mut self, mark: impl IntoElement) -> Self {
        self.mark = Some(mark.into_any_element());
        self
    }
}

impl RenderOnce for Pill {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // The size: text size and the room around it.
        let pill = div().flex_none().rounded_full();
        let pill = match self.size {
            PillSize::Compact => pill.px_1p5().text_size(px(COMPACT_TEXT_SIZE)),
            PillSize::Small => pill.px_1p5().text_xs(),
            PillSize::Roomy => pill.px_2().py_0p5().text_xs(),
        };
        // The style: border, fill, text color and the weight it implies.
        let (pill, style_weight) = match self.style {
            PillStyle::Outlined => (
                pill.border_1()
                    .border_color(theme::border_strong())
                    .text_color(theme::muted()),
                None,
            ),
            PillStyle::Tinted(color) => {
                (pill.border_1().border_color(color).text_color(color), None)
            }
            PillStyle::Filled => (pill.bg(theme::surface()).text_color(theme::muted()), None),
            PillStyle::Framed => (
                pill.border_1()
                    .border_color(theme::border())
                    .bg(theme::surface())
                    .text_color(theme::muted()),
                None,
            ),
            PillStyle::Accent => (
                pill.bg(theme::selected()).text_color(theme::accent()),
                Some(FontWeight::MEDIUM),
            ),
        };
        // A weight given to the pill wins over its style's.
        let pill = match self.weight.or(style_weight) {
            Some(weight) => pill.font_weight(weight),
            None => pill,
        };
        // Label color can carry meaning without making the capsule's outline stronger.
        let pill = pill.when_some(self.text_color, Styled::text_color);
        // A mark sits before the text, centered on it.
        match self.mark {
            Some(mark) => pill
                .flex()
                .items_center()
                .gap_1p5()
                .child(mark)
                .child(self.text),
            None => pill.child(self.text),
        }
    }
}
