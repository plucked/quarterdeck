//! `SegmentedControl`: a row of mutually exclusive options, like a toolbar
//! toggle between "Unified" and "Split".

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

use crate::theme;

/// What an option does when clicked.
type SelectHandler = Box<dyn Fn(&mut Window, &mut App)>;

/// One option of a [`SegmentedControl`].
struct SegmentOption {
    label: SharedString,
    is_selected: bool,
    on_select: SelectHandler,
}

/// A bordered row of options of which one is selected; clicking an option
/// calls its handler (the owner then re-renders with the new selection).
#[derive(IntoElement)]
pub struct SegmentedControl {
    id: ElementId,
    options: Vec<SegmentOption>,
}

impl SegmentedControl {
    /// An empty control; `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
        }
    }

    /// Adds an option, shown selected when `is_selected`.
    pub fn option(
        mut self,
        label: impl Into<SharedString>,
        is_selected: bool,
        on_select: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        self.options.push(SegmentOption {
            label: label.into(),
            is_selected,
            on_select: Box::new(on_select),
        });
        self
    }
}

impl RenderOnce for SegmentedControl {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .flex()
            .flex_none()
            .gap_0p5()
            .p_0p5()
            .rounded_md()
            .bg(theme::sidebar())
            .border_1()
            .border_color(theme::border())
            .text_xs()
            .children(self.options.into_iter().enumerate().map(|(index, option)| {
                div()
                    .id(index)
                    .px_2()
                    .py_0p5()
                    .rounded(gpui::px(4.))
                    .cursor_pointer()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .when(option.is_selected, |segment| {
                        segment.bg(theme::surface()).text_color(theme::text())
                    })
                    .when(!option.is_selected, |segment| {
                        segment
                            .text_color(theme::muted())
                            .hover(|style| style.text_color(theme::text()))
                    })
                    .on_click(move |_, window, cx| (option.on_select)(window, cx))
                    .child(option.label)
            }))
    }
}
