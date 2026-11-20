//! A compact text field that expands horizontally while hovered.

use std::time::Instant;

use gpui::{
    AnyElement, App, AvailableSpace, ElementId, IntoElement, Pixels, RenderOnce, ShapedLine,
    SharedString, Window, canvas, div, prelude::*, size,
};

use super::animated_reveal::RevealMotion;

type HoverHandler = Box<dyn Fn(bool, &mut App)>;

/// Reveals a text line and any trailing content in place from an ellipsized preview.
#[derive(IntoElement)]
pub struct HoverExpand {
    id: ElementId,
    text: SharedString,
    suffix: Box<dyn Fn() -> AnyElement>,
    on_hover: Option<HoverHandler>,
}

impl HoverExpand {
    /// Shows as much text as fits and reveals the suffix on hover.
    pub fn new<E: IntoElement>(
        id: impl Into<ElementId>,
        text: SharedString,
        suffix: impl Fn() -> E + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            text,
            suffix: Box::new(move || suffix().into_any_element()),
            on_hover: None,
        }
    }

    /// Reports the hover change so neighboring controls can make room.
    #[must_use]
    pub fn on_hover(mut self, handler: impl Fn(bool, &mut App) + 'static) -> Self {
        self.on_hover = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for HoverExpand {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // A compact header always shows a single text line.
        let text = if self.text.contains(['\r', '\n']) {
            self.text.replace(['\r', '\n'], " ").into()
        } else {
            self.text
        };
        // Read motion without restarting it on a repaint. A hover change
        // reverses from the currently visible width.
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| RevealMotion::new(false));
        let now = Instant::now();
        let motion = state.read(cx);
        let amount = motion.value(now);
        if motion.is_active(now) {
            window.request_animation_frame();
        }
        let on_hover = self.on_hover;
        let line_id = self.id.clone();
        div()
            .id(self.id)
            .flex()
            .items_center()
            .when(amount <= 0., Styled::min_w_0)
            .on_hover(move |hovered, _, cx| {
                state.update(cx, |motion, cx| {
                    motion.set_expanded(*hovered, Instant::now());
                    cx.notify();
                });
                if let Some(handler) = &on_hover {
                    handler(*hovered, cx);
                }
            })
            .child(render_line(text, line_id, amount, window))
            .when(amount > 0., |field| {
                // Measure a separate copy so the visible element can be laid out with its parent.
                let width = (self.suffix)()
                    .layout_as_root(
                        size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
                        window,
                        cx,
                    )
                    .width;
                field.child(
                    div()
                        .flex_none()
                        .w(width * amount)
                        .overflow_hidden()
                        .child((self.suffix)()),
                )
            })
    }
}

/// Uses the untruncated line for flex sizing and the final allocated width for painting.
fn render_line(
    text: SharedString,
    id: ElementId,
    amount: f32,
    window: &Window,
) -> impl IntoElement {
    let width = shape_line(text.clone(), window).width.ceil();
    div()
        .debug_selector(move || format!("{id}-line"))
        .w(width)
        .h(window.line_height())
        .min_w(width * amount)
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, (), window, cx| {
                    if let Some(line) = fit_line(&text, bounds.size.width, window) {
                        line.paint(bounds.origin, bounds.size.height, window, cx)
                            .ok();
                    }
                },
            )
            .size_full(),
        )
}

/// Shapes the line in the field's current style, including its hover color.
fn shape_line(text: SharedString, window: &Window) -> ShapedLine {
    let style = window.text_style();
    let run = style.to_run(text.len());
    window.text_system().shape_line(
        text,
        style.font_size.to_pixels(window.rem_size()),
        &[run],
        None,
    )
}

/// Reserves room for all three dots and shapes the shortened line before painting it.
fn fit_line(text: &SharedString, width: Pixels, window: &Window) -> Option<ShapedLine> {
    let full = shape_line(text.clone(), window);
    if full.width <= width {
        return Some(full);
    }
    let ellipsis = shape_line("...".into(), window);
    if ellipsis.width > width {
        return None;
    }

    // The dot glyphs can change the preceding glyph spacing. Check the actual shaped width.
    let end = full.index_for_x(width - ellipsis.width).unwrap_or(0);
    let mut prefix = text.get(..end).unwrap_or_default().trim_end();
    while let Some((last, _)) = prefix.char_indices().next_back() {
        let shortened = shape_line(format!("{prefix}...").into(), window);
        if shortened.width <= width {
            return Some(shortened);
        }
        prefix = prefix[..last].trim_end();
    }
    Some(ellipsis)
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;

    /// The painted preview includes three dots inside the exact allocated width.
    #[gpui::test]
    fn shortened_line_reserves_room_for_all_three_dots(cx: &mut TestAppContext) {
        cx.add_empty_window().update(|window, _| {
            let text: SharedString = "feat: switch the transport to the new backend".into();
            let full_width = shape_line(text.clone(), window).width;
            let preview_width = full_width / 2.;
            let preview = fit_line(&text, preview_width, window).unwrap();
            assert!(preview.text.starts_with("feat: "));
            assert!(preview.text.ends_with("..."));
            assert!(preview.width <= preview_width);
            assert_eq!(fit_line(&text, full_width, window).unwrap().text, text);
        });
    }
}
