use gpui::{
    App, AvailableSpace, Bounds, ElementId, ElementInputHandler, Entity, Focusable,
    GlobalElementId, Hsla, IntoElement, LayoutId, PaintQuad, Pixels, SharedString, Size, Style,
    TextAlign, TextRun, TextStyle, Window, fill, point, prelude::*, px, relative, size,
};

use super::text_area::TextArea;
use super::text_area_layout::TextAreaLayout;
use crate::theme;

/// Width of the caret.
const CARET_WIDTH: f32 = 1.5;
/// Width used to measure before the real width is known.
const FALLBACK_WIDTH: f32 = 400.;

/// The element of a [`TextArea`]: as tall as its wrapped text, painting the
/// selection, the text and the caret, and taking the system's text input
/// while the area is focused.
pub struct TextAreaElement {
    area: Entity<TextArea>,
}

impl TextAreaElement {
    pub fn new(area: Entity<TextArea>) -> Self {
        Self { area }
    }
}

/// What prepaint computed for paint.
pub struct TextAreaPaint {
    layout: TextAreaLayout,
    selection: Vec<PaintQuad>,
    caret: Option<PaintQuad>,
}

impl IntoElement for TextAreaElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextAreaElement {
    type RequestLayoutState = ();
    type PrepaintState = TextAreaPaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // As wide as the parent, as tall as the text wrapped to that width.
        let (text, color) = shown_text(self.area.read(cx), window);
        let text_style = window.text_style();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let layout_id =
            window.request_measured_layout(style, move |known, available, window, _| {
                let width = known.width.unwrap_or(match available.width {
                    AvailableSpace::Definite(width) => width,
                    AvailableSpace::MinContent | AvailableSpace::MaxContent => px(FALLBACK_WIDTH),
                });
                let layout = shape(&text, color, &text_style, width, window);
                Size {
                    width,
                    height: layout.height,
                }
            });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        // Shape for the real width, then place the selection and the caret.
        let area = self.area.read(cx);
        let (text, color) = shown_text(area, window);
        let layout = shape(
            &text,
            color,
            &window.text_style(),
            bounds.size.width,
            window,
        );
        let is_empty = area.text().is_empty();
        let selected = area.selected_range.clone();
        let selection = if selected.is_empty() || is_empty {
            Vec::new()
        } else {
            layout
                .selection_rects(&selected, bounds.size.width)
                .into_iter()
                .map(|rect| {
                    fill(
                        Bounds::new(bounds.origin + rect.origin, rect.size),
                        theme::text_selection(),
                    )
                })
                .collect()
        };
        let caret_offset = if is_empty { 0 } else { area.cursor_offset() };
        let caret_position = layout.position_for_offset(caret_offset);
        let caret = (selected.is_empty() && area.is_focused(window)).then(|| {
            fill(
                Bounds::new(
                    bounds.origin + caret_position,
                    size(px(CARET_WIDTH), layout.line_height),
                ),
                theme::accent(),
            )
        });
        TextAreaPaint {
            layout,
            selection,
            caret,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Typing reaches the area through the system's input handler while it is focused.
        let focus = self.area.read(cx).focus_handle(cx);
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.area.clone()),
            cx,
        );

        // The selection under the text, then each line, then the caret.
        for quad in prepaint.selection.drain(..) {
            window.paint_quad(quad);
        }
        let layout = &prepaint.layout;
        for (line, top) in layout.lines.iter().zip(&layout.tops) {
            line.paint(
                point(bounds.origin.x, bounds.origin.y + *top),
                layout.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .ok();
        }
        if let Some(caret) = prepaint.caret.take() {
            window.paint_quad(caret);
        }

        // Keep this frame's layout for hit tests and the input method.
        let layout = std::mem::replace(
            &mut prepaint.layout,
            TextAreaLayout {
                lines: Vec::new(),
                ranges: Vec::new(),
                tops: Vec::new(),
                line_height: px(0.),
                height: px(0.),
            },
        );
        self.area.update(cx, |area, _| {
            area.last_layout = Some(layout);
            area.last_bounds = Some(bounds);
        });
    }
}

/// The text to draw: the text, or the placeholder (muted) while empty.
fn shown_text(area: &TextArea, window: &Window) -> (SharedString, Hsla) {
    if area.text().is_empty() {
        (area.placeholder().clone(), theme::muted().into())
    } else {
        (
            SharedString::from(area.text().to_owned()),
            window.text_style().color,
        )
    }
}

/// Shapes `text` line by line, wrapped to `width`.
fn shape(
    text: &str,
    color: Hsla,
    style: &TextStyle,
    width: Pixels,
    window: &mut Window,
) -> TextAreaLayout {
    let font_size = style.font_size.to_pixels(window.rem_size());
    let line_height = window.line_height();
    let mut layout = TextAreaLayout {
        lines: Vec::new(),
        ranges: Vec::new(),
        tops: Vec::new(),
        line_height,
        height: px(0.),
    };

    // One wrapped line per `\n`-separated line, stacked from the top.
    let mut start = 0;
    for line_text in text.split('\n') {
        let run = TextRun {
            len: line_text.len(),
            font: style.font(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped = window
            .text_system()
            .shape_text(
                SharedString::from(line_text.to_owned()),
                font_size,
                &[run],
                Some(width),
                None,
            )
            .ok()
            .and_then(|mut lines| (!lines.is_empty()).then(|| lines.remove(0)));
        let Some(shaped) = shaped else {
            continue;
        };
        layout.tops.push(layout.height);
        layout.height += shaped.size(line_height).height;
        layout.ranges.push(start..start + line_text.len());
        layout.lines.push(shaped);
        start += line_text.len() + 1;
    }
    layout.height = layout.height.max(line_height);
    layout
}
