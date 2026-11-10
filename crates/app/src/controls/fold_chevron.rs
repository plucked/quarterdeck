//! `FoldChevron`: the button that folds or unfolds something (a repository
//! or stack in the sidebar, a folder in the file tree, a file in the diff,
//! the description), big enough to see and to hit.

use gpui::{App, ClickEvent, ElementId, IntoElement, RenderOnce, Window, div, prelude::*, px};

use crate::theme;

/// Edge length of the clickable square; rows without a chevron leave this
/// much room so names line up.
pub const FOLD_CHEVRON_SIZE: f32 = 24.;
/// Size of the triangle glyph.
const GLYPH_SIZE: f32 = 10.;

/// What a click on the chevron runs.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Whether the group the chevron belongs to is folded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoldState {
    Folded,
    Unfolded,
}

impl FoldState {
    pub fn of(is_folded: bool) -> Self {
        if is_folded {
            Self::Folded
        } else {
            Self::Unfolded
        }
    }
}

/// A triangle pointing right (folded) or down (unfolded) in a square that
/// tints on hover. Without a click handler it only shows the state, and
/// its row handles the click.
#[derive(IntoElement)]
pub struct FoldChevron {
    id: ElementId,
    state: FoldState,
    on_click: Option<ClickHandler>,
}

impl FoldChevron {
    pub fn new(id: impl Into<ElementId>, state: FoldState) -> Self {
        Self {
            id: id.into(),
            state,
            on_click: None,
        }
    }

    /// Folds or unfolds on its own click, which does not reach the row.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for FoldChevron {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let glyph = match self.state {
            FoldState::Folded => "▶",
            FoldState::Unfolded => "▼",
        };
        let button = div()
            .id(self.id)
            .flex_none()
            .size(px(FOLD_CHEVRON_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .text_size(px(GLYPH_SIZE))
            .text_color(theme::muted())
            .cursor_pointer()
            .hover(|style| style.bg(theme::hover()).text_color(theme::text()))
            .child(glyph);
        match self.on_click {
            Some(handler) => button.on_click(move |event, window, cx| {
                cx.stop_propagation();
                handler(event, window, cx);
            }),
            None => button,
        }
    }
}
