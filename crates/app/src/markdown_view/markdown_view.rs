use std::sync::Arc;

use gpui::{
    App, ClipboardItem, CursorStyle, DispatchPhase, ElementId, Entity, IntoElement, MouseButton,
    MouseMoveEvent, MouseUpEvent, RenderOnce, SharedString, Window, canvas, div, prelude::*,
};
use qc_markdown::MarkdownDocument;

use super::block_rendering::render_blocks;
use super::markdown_selection::MarkdownSelection;
use super::selectable_texts::SelectableTexts;
use crate::app_commands::Copy;

/// A rendered Markdown document, sized to its container's width, whose text
/// can be selected with the mouse and copied with cmd-c.
///
/// Built each frame by its parent view, which parses the source once and
/// keeps the document in an `Arc`: elements must own what they draw, so each
/// frame clones the `Arc` instead of the document. `id` must be unique among
/// its siblings. The selection lives as long as the same document is shown
/// under the same `id`; another document starts with none.
#[derive(IntoElement)]
pub struct MarkdownView {
    id: SharedString,
    document: Arc<MarkdownDocument>,
}

impl MarkdownView {
    pub fn new(id: impl Into<SharedString>, document: Arc<MarkdownDocument>) -> Self {
        Self {
            id: id.into(),
            document,
        }
    }
}

impl RenderOnce for MarkdownView {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Selection state per view id and document, so a new document starts fresh.
        let state_key = ElementId::named_usize(self.id.clone(), Arc::as_ptr(&self.document).addr());
        let state = window.use_keyed_state(state_key, cx, |_, cx| MarkdownSelection::new(cx));

        // Render the blocks with the current selection, then hand their
        // layouts to the state for the next mouse events.
        let mut texts = SelectableTexts::new(state.read(cx).selected_range());
        let blocks = render_blocks(self.document.blocks(), &mut texts);
        state.update(cx, |state, _| state.replace_segments(texts.into_segments()));
        let focus = state.read(cx).focus().clone();

        div()
            .id(self.id)
            .track_focus(&focus)
            .relative()
            .cursor(CursorStyle::IBeam)
            .on_mouse_down(MouseButton::Left, {
                let state = state.clone();
                move |event, window, cx| {
                    state.update(cx, |state, cx| {
                        state.begin(event.position, event.click_count, window, cx);
                    });
                }
            })
            .on_action({
                let state = state.clone();
                move |_: &Copy, _window, cx| {
                    if let Some(text) = state.read(cx).selected_text() {
                        cx.write_to_clipboard(ClipboardItem::new_string(text));
                    }
                }
            })
            .flex()
            .flex_col()
            .gap_3()
            .children(blocks)
            .child(drag_listener(state).absolute().size_full())
    }
}

/// An empty overlay whose paint listens to mouse moves and releases
/// anywhere in the window, so a drag keeps selecting outside the document.
fn drag_listener(state: Entity<MarkdownSelection>) -> gpui::Canvas<()> {
    canvas(
        |_, _, _| {},
        move |_, (), window, _| {
            let moving = state.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
                let is_drag = event.pressed_button == Some(MouseButton::Left);
                if phase == DispatchPhase::Bubble && is_drag && moving.read(cx).is_dragging() {
                    moving.update(cx, |state, cx| state.extend(event.position, cx));
                }
            });
            window.on_mouse_event(move |event: &MouseUpEvent, phase, _window, cx| {
                if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                    state.update(cx, MarkdownSelection::finish);
                }
            });
        },
    )
}
