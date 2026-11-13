//! Native NumberFlow-style numeric labels. The final string always owns
//! layout; only its paint changes. Ordinary text keeps GPUI's wrapping,
//! ellipsis, font, alignment and color. No timer runs for an idle label.

mod transition;

#[cfg(test)]
mod tests;

use std::{panic::Location, sync::Arc, time::Instant};

use gpui::{
    App, Bounds, ContentMask, Element, ElementId, FontFeatures, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, SharedString, StyledText, TextAlign,
    TextStyle, Window, point, px, size,
};

use super::value_motion::reduce_motion;
use transition::{Cell, Key, Transition, face, parts};

/// A numeric value or a short formatted label (`+12`, `3 of 14`, `1.2 MB`).
/// Give it a stable id within its parent; never use its changing value as
/// the id. It displays the first value immediately and rolls on updates.
pub struct NumberFlow {
    id: ElementId,
    label: SharedString,
    text: StyledText,
}

/// Paint data for the native element; not part of the control's API.
pub struct NumberFlowPaint(Option<Vec<Cell>>);

impl NumberFlow {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            id: id.into(),
            text: StyledText::new(label.clone()),
            label,
        }
    }

    /// For shared controls without their own id. Their constructor must
    /// propagate `#[track_caller]`; repeated items still need a keyed parent.
    #[track_caller]
    pub fn label(label: impl Into<SharedString>) -> Self {
        Self::new(ElementId::CodeLocation(*Location::caller()), label)
    }
}

impl IntoElement for NumberFlow {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for NumberFlow {
    type RequestLayoutState = ();
    type PrepaintState = NumberFlowPaint;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        // Equal advances keep changing digits from nudging their neighbors.
        let style = numeric_style(window);
        self.text =
            StyledText::new(self.label.clone()).with_runs(vec![style.to_run(self.label.len())]);
        self.text.request_layout(None, inspector, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.text
            .prepaint(None, inspector, bounds, state, window, cx);
        if !self.label.bytes().any(|byte| byte.is_ascii_digit()) {
            return NumberFlowPaint(None);
        }
        let shown = self.text.layout().text();
        let cells = layout_cells(self.text.layout(), &shown, bounds, window);
        let animated = !reduce_motion();
        NumberFlowPaint(window.with_element_state(
            id.expect("NumberFlow has an id"),
            |transition: Option<Transition>, _| {
                let mut transition = transition.unwrap_or_default();
                let now = Instant::now();
                transition.update(&shown, cells, now, animated);
                let frame = transition.is_active(now).then(|| transition.frame(now));
                (frame, transition)
            },
        ))
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        state: &mut (),
        frame: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(frame) = &frame.0 else {
            self.text
                .paint(None, inspector, bounds, state, &mut (), window, cx);
            return;
        };
        let style = numeric_style(window);
        let height = window.line_height();
        let mask = ContentMask {
            bounds: animation_bounds(bounds, frame, height),
        };
        window.with_content_mask(Some(mask), |window| {
            for cell in frame.iter().filter(|cell| cell.opacity > 0.001) {
                let origin = bounds.origin + point(px(cell.x), px(cell.y));
                if let Some(wheel) = cell.wheel {
                    // Two adjacent faces of a ten-digit reel. Clip each reel
                    // vertically so it cannot paint over the rows above/below.
                    let fraction = wheel - wheel.floor();
                    let mask = ContentMask {
                        bounds: Bounds::new(origin, size(px(cell.width), height)),
                    };
                    window.with_content_mask(Some(mask), |window| {
                        for step in [0_u8, 1] {
                            let digit = face(wheel, step);
                            let offset = (f32::from(step) - fraction) * height;
                            paint_part(
                                &digit,
                                origin + point(px(0.), offset),
                                cell.opacity,
                                &style,
                                window,
                                cx,
                            );
                        }
                    });
                } else {
                    paint_part(&cell.text, origin, cell.opacity, &style, window, cx);
                }
            }
        });
        // Paint only happens for visible elements. No tasks or permanent
        // animation loops are created for virtualized/offscreen numbers.
        window.request_animation_frame();
    }
}

/// Keep outgoing glyphs visible when the final label is narrower or shorter.
/// The window still intersects this mask with every ancestor's clip, and
/// each digit reel retains its own vertical mask.
fn animation_bounds(bounds: Bounds<Pixels>, frame: &[Cell], line_height: Pixels) -> Bounds<Pixels> {
    frame
        .iter()
        .filter(|cell| cell.opacity > 0.001)
        .fold(bounds, |paint_bounds, cell| {
            paint_bounds.union(&Bounds::new(
                bounds.origin + point(px(cell.x), px(cell.y)),
                size(px(cell.width), line_height),
            ))
        })
}

/// Keep native word wrapping and truncation: split literal text only at
/// actual shaped line boundaries. Numeric columns are matched by place,
/// even when several independent counters share a paragraph.
fn layout_cells(
    layout: &gpui::TextLayout,
    text: &str,
    bounds: Bounds<Pixels>,
    window: &Window,
) -> Vec<Cell> {
    let style = numeric_style(window);
    let mut cells = Vec::new();
    for part in parts(text) {
        let mut fragments: Vec<(usize, usize, gpui::Point<Pixels>)> = Vec::new();
        for (offset, ch) in text[part.range.clone()].char_indices() {
            let start = part.range.start + offset;
            if ch == '\n' {
                continue;
            }
            let Some(mut position) = layout.position_for_index(start) else {
                continue;
            };
            // GPUI returns the caret at the preceding line's end for a
            // wrap boundary. A glyph at that boundary belongs to the next
            // line; its following caret supplies that line's y coordinate.
            if let Some(next) = layout.position_for_index(start + ch.len_utf8())
                && next.y > position.y
            {
                position = point(bounds.left(), next.y);
            }
            if let Some((_, end, previous)) = fragments.last_mut()
                && previous.y == position.y
                && *end == start
            {
                *end += ch.len_utf8();
            } else {
                fragments.push((start, start + ch.len_utf8(), position));
            }
        }
        for (fragment, (start, end, position)) in fragments.into_iter().enumerate() {
            let text = &text[start..end];
            let width = window
                .text_system()
                .shape_line(
                    text.to_owned().into(),
                    style.font_size.to_pixels(window.rem_size()),
                    &[style.to_run(text.len())],
                    None,
                )
                .width;
            let key = match part.key {
                Key::Text { run, .. } => Key::Text { run, fragment },
                ref key => key.clone(),
            };
            cells.push(Cell {
                key,
                text: text.to_owned(),
                wheel: part.digit.map(f32::from),
                x: f32::from(position.x - bounds.left()),
                y: f32::from(position.y - bounds.top()),
                width: f32::from(width),
                opacity: 1.,
            });
        }
    }
    let alignment = window.text_style().text_align;
    if alignment != TextAlign::Left {
        let offsets: Vec<_> = cells
            .iter()
            .map(|cell| {
                let width = cells
                    .iter()
                    .filter(|other| (other.y - cell.y).abs() < 0.5)
                    .map(|other| other.x + other.width)
                    .fold(0., f32::max);
                let space = f32::from(bounds.size.width) - width;
                if alignment == TextAlign::Center {
                    space / 2.
                } else {
                    space
                }
            })
            .collect();
        for (cell, offset) in cells.iter_mut().zip(offsets) {
            cell.x += offset;
        }
    }
    cells
}

fn numeric_style(window: &Window) -> TextStyle {
    let mut style = window.text_style();
    let mut features = style.font_features.0.as_ref().clone();
    features.retain(|(tag, _)| tag != "tnum");
    features.push(("tnum".to_owned(), 1));
    style.font_features = FontFeatures(Arc::new(features));
    style
}

fn paint_part(
    text: &str,
    origin: gpui::Point<Pixels>,
    opacity: f32,
    style: &TextStyle,
    window: &mut Window,
    cx: &mut App,
) {
    let mut run = style.to_run(text.len());
    run.color.a *= opacity;
    let line = window.text_system().shape_line(
        text.to_owned().into(),
        style.font_size.to_pixels(window.rem_size()),
        &[run],
        None,
    );
    line.paint(origin, window.line_height(), window, cx).ok();
}
