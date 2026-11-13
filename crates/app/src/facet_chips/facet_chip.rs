use gpui::{
    AnyElement, App, ClickEvent, FontWeight, IntoElement, RenderOnce, SharedString, Window, div,
    prelude::*,
};
use qd_file_tree::{FacetChoice, FacetCount};

use crate::controls::{NumberFlow, TextTooltip};
use crate::theme;

/// Opacity of an open chip that would show nothing given the other rows.
const EMPTY_CHIP_OPACITY: f32 = 0.45;

/// What a chip does when clicked.
type ChipClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// One filter chip with what picking it would show: picked chips stand
/// out, a hidden one is struck through (with how many it hides), and one
/// that would show nothing is dimmed. An optional mark goes before the
/// label (an author's avatar, a label's color).
#[derive(IntoElement)]
pub struct FacetChip {
    id: SharedString,
    label: SharedString,
    choice: FacetChoice,
    /// The number after the label: what it would show, or for a hidden
    /// chip how many it hides.
    number: usize,
    /// Whether picking it would show nothing given the other rows.
    shows_nothing: bool,
    leading: Option<AnyElement>,
    tooltip: Option<SharedString>,
    on_click: Option<ChipClickHandler>,
}

impl FacetChip {
    /// The chip `id` labelled `label` for `count`, picked, hidden or open as `choice` says.
    pub fn new<T>(
        id: impl Into<SharedString>,
        label: impl Into<SharedString>,
        count: &FacetCount<T>,
        choice: FacetChoice,
    ) -> Self {
        let number = match choice {
            FacetChoice::Hidden => count.total,
            FacetChoice::Open | FacetChoice::Only => count.matching,
        };
        Self {
            id: id.into(),
            label: label.into(),
            choice,
            number,
            shows_nothing: count.matching == 0,
            leading: None,
            tooltip: None,
            on_click: None,
        }
    }

    /// Puts `leading` before the label, when there is one.
    #[must_use]
    pub fn leading(mut self, leading: Option<AnyElement>) -> Self {
        self.leading = leading;
        self
    }

    /// Explains a value whose label alone does not convey its meaning.
    #[must_use]
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for FacetChip {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let chip = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_1()
            .px_2()
            .py_0p5()
            .rounded_full()
            .border_1()
            .text_xs()
            .cursor_pointer()
            .children(self.leading)
            .min_w_0()
            .max_w_full()
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .child(NumberFlow::label(self.label)),
            )
            .child(
                div()
                    .text_color(theme::subtle())
                    .child(NumberFlow::label(self.number.to_string())),
            )
            .when_some(self.tooltip, |chip, text| {
                chip.tooltip(TextTooltip::builder(text))
            })
            .when_some(self.on_click, |chip, handler| chip.on_click(handler));
        match self.choice {
            FacetChoice::Only => chip
                .border_color(theme::accent())
                .bg(theme::selected())
                .text_color(theme::text())
                .font_weight(FontWeight::MEDIUM),
            FacetChoice::Hidden => chip
                .border_color(theme::border())
                .text_color(theme::subtle())
                .line_through(),
            FacetChoice::Open => chip
                .border_color(theme::border())
                .bg(theme::surface())
                .text_color(theme::muted())
                .hover(|style| {
                    style
                        .border_color(theme::border_strong())
                        .text_color(theme::text())
                })
                .when(self.shows_nothing, |chip| chip.opacity(EMPTY_CHIP_OPACITY)),
        }
    }
}
