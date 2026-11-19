//! `SegmentedControl`: a row of mutually exclusive options, like a toolbar
//! toggle between "Unified" and "Split".

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, RenderOnce, SharedString, Styled, Window,
    div, prelude::*, px,
};

use super::NumberFlow;
use crate::theme;

/// Corner radius of an option in the bordered look.
const SEGMENT_CORNER_RADIUS: f32 = 4.;
/// Room between the selected option's detail and the option's right edge:
/// the detail sits close to the edge, like an option inside the option.
const DETAIL_TRAILING_ROOM: f32 = 3.;

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

/// What the control and its selected option are drawn with.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlLook {
    /// A bordered box; the selected option filled.
    Bordered,
    /// A borderless capsule; the selected option a filled capsule inside it.
    Quiet,
    /// A capsule cut into a filled surface: the track in the background
    /// color, the selected option in the hover fill, so both stand out from
    /// the surface around them.
    Inset,
}

/// One option of a [`SegmentedControl`].
struct SegmentOption {
    label: SharedString,
    is_selected: bool,
    on_select: SelectHandler,
}

/// A row of options of which one is selected, bordered by default, a
/// borderless capsule with [`Self::quiet`] or one cut into a filled surface
/// with [`Self::inset`]; clicking an option calls its handler (the owner
/// then re-renders with the new selection) and goes no further, like a
/// [`Button`](crate::controls::Button) click.
#[derive(IntoElement)]
pub struct SegmentedControl {
    id: ElementId,
    options: Vec<SegmentOption>,
    width: ControlWidth,
    size: ControlSize,
    look: ControlLook,
    selected_detail: Option<AnyElement>,
}

impl SegmentedControl {
    /// An empty control; `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            width: ControlWidth::Fit,
            size: ControlSize::Small,
            look: ControlLook::Bordered,
            selected_detail: None,
        }
    }

    /// Spans the container's width, the options sharing it equally.
    #[must_use]
    pub fn fill_width(mut self) -> Self {
        self.width = ControlWidth::Fill;
        self
    }

    /// Draws it larger, for a view's main switch.
    #[must_use]
    pub fn large(mut self) -> Self {
        self.size = ControlSize::Large;
        self
    }

    /// A borderless capsule for view navigation on an open canvas.
    #[must_use]
    pub fn quiet(mut self) -> Self {
        self.look = ControlLook::Quiet;
        self
    }

    /// A capsule for a control that sits on a filled surface, such as inside
    /// another control's selected option.
    #[must_use]
    pub fn inset(mut self) -> Self {
        self.look = ControlLook::Inset;
        self
    }

    /// Shows `detail` inside the selected option, after its label: a choice
    /// that belongs to that option only, such as a small control of its own.
    /// A click on the detail that it does not handle itself selects the
    /// option.
    #[must_use]
    pub fn selected_detail(mut self, detail: impl IntoElement) -> Self {
        self.selected_detail = Some(detail.into_any_element());
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
        let look = self.look;
        let size = self.size;
        let width = self.width;
        let mut selected_detail = self.selected_detail;
        // The box around the options.
        let control = div()
            .id(self.id)
            .flex()
            .flex_none()
            .when(width == ControlWidth::Fill, Styled::w_full);
        let control = match look {
            ControlLook::Bordered => control
                .gap_0p5()
                .p_0p5()
                .rounded_md()
                .bg(theme::sidebar())
                .border_1()
                .border_color(theme::border()),
            ControlLook::Quiet => control.gap_0p5().p_0p5().rounded_full().bg(theme::panel()),
            ControlLook::Inset => control
                .gap_0p5()
                .p_0p5()
                .rounded_full()
                .bg(theme::background()),
        };
        let control = match size {
            ControlSize::Small => control.text_xs(),
            ControlSize::Large => control.text_sm(),
        };
        control.children(self.options.into_iter().enumerate().map(|(index, option)| {
            // Only the selected option carries the detail.
            let detail = selected_detail.take_if(|_| option.is_selected);
            // The label brings the option's room, so a detail beside it
            // does not make the option taller.
            let label = div().child(NumberFlow::new("segment-label", option.label));
            let label = match size {
                ControlSize::Small => label.px_2().py_0p5(),
                ControlSize::Large => label.px_4().py_1(),
            };
            let segment = div()
                .id(index)
                .flex()
                .items_center()
                .cursor_pointer()
                .font_weight(FontWeight::MEDIUM)
                .when(width == ControlWidth::Fill, |segment| {
                    segment.flex_1().justify_center()
                });
            let segment = match look {
                ControlLook::Bordered => segment.rounded(px(SEGMENT_CORNER_RADIUS)),
                ControlLook::Quiet | ControlLook::Inset => segment.rounded_full(),
            };
            let segment = match (option.is_selected, look) {
                (true, ControlLook::Bordered | ControlLook::Quiet) => {
                    segment.bg(theme::surface()).text_color(theme::text())
                }
                (true, ControlLook::Inset) => segment.bg(theme::hover()).text_color(theme::text()),
                (false, _) => segment
                    .text_color(theme::muted())
                    .hover(|style| style.text_color(theme::text())),
            };
            segment
                // A clickable row around the control ignores this click.
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    (option.on_select)(window, cx);
                })
                .child(label)
                .when_some(detail, |segment, detail| {
                    segment.pr(px(DETAIL_TRAILING_ROOM)).child(detail)
                })
        }))
    }
}
