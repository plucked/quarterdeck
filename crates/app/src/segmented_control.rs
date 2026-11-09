//! `SegmentedControl`: a row of mutually exclusive options, like a toolbar
//! toggle between "Unified" and "Split".

use gpui::{
    App, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window, div, prelude::*,
};

use crate::theme;

/// What an option does when clicked.
type SelectHandler = Box<dyn Fn(&mut Window, &mut App)>;

/// How wide the control is.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlWidth {
    /// As wide as its labels.
    Fit,
    /// As wide as its container, the options sharing it equally.
    Fill,
}

/// How large the control's labels and padding are.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlSize {
    /// Small labels: options inside panels.
    Small,
    /// Body-size labels with more room: a view's main switch.
    Large,
}

/// One option of a [`SegmentedControl`].
struct SegmentOption {
    label: SharedString,
    is_selected: bool,
    on_select: SelectHandler,
}

/// A bordered row of options of which one is selected; clicking an option
/// calls its handler (the owner then re-renders with the new selection) and
/// goes no further, like a [`Button`](crate::button::Button) click.
#[derive(IntoElement)]
pub struct SegmentedControl {
    id: ElementId,
    options: Vec<SegmentOption>,
    width: ControlWidth,
    size: ControlSize,
}

impl SegmentedControl {
    /// An empty control; `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            width: ControlWidth::Fit,
            size: ControlSize::Small,
        }
    }

    /// Spans the container's width, the options sharing it equally.
    #[must_use]
    pub fn fill_width(mut self) -> Self {
        self.width = ControlWidth::Fill;
        self
    }

    /// Draws it larger, for a view's main switch (the detail's Diff / Brief).
    #[must_use]
    pub fn large(mut self) -> Self {
        self.size = ControlSize::Large;
        self
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
            .when(self.width == ControlWidth::Fill, Styled::w_full)
            .gap_0p5()
            .p_0p5()
            .rounded_md()
            .bg(theme::sidebar())
            .border_1()
            .border_color(theme::border())
            .map(|control| match self.size {
                ControlSize::Small => control.text_xs(),
                ControlSize::Large => control.text_sm(),
            })
            .children(self.options.into_iter().enumerate().map(|(index, option)| {
                div()
                    .id(index)
                    .map(|segment| match self.size {
                        ControlSize::Small => segment.px_2().py_0p5(),
                        ControlSize::Large => segment.px_4().py_1(),
                    })
                    .when(self.width == ControlWidth::Fill, |segment| {
                        segment.flex_1().flex().justify_center()
                    })
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
                    // The click is the option's alone: a clickable row around the
                    // control (the detail's title row folds the description) ignores it.
                    .on_click(move |_, window, cx| {
                        cx.stop_propagation();
                        (option.on_select)(window, cx);
                    })
                    .child(option.label)
            }))
    }
}
