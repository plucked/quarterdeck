//! `TreeLocateButton`: the icon button in a diff file header that shows the
//! file in the file tree, drawn as a small folder tree with the file's row
//! in the accent color.

use gpui::{App, ClickEvent, ElementId, IntoElement, RenderOnce, Window, div, prelude::*, px};

use crate::theme;

/// Size of the clickable square.
const BUTTON_SIZE: f32 = 22.;
/// Width and height of the drawn tree.
const GLYPH_WIDTH: f32 = 14.;
const GLYPH_HEIGHT: f32 = 12.;
/// The folder bar at the top of the tree.
const FOLDER_WIDTH: f32 = 7.;
const FOLDER_HEIGHT: f32 = 3.;
/// Where the rail runs down from the folder, from the glyph's left edge.
const RAIL_LEFT: f32 = 2.;
/// Where each file row starts, from the glyph's left edge, and its height.
const FILE_LEFT: f32 = 6.;
const FILE_HEIGHT: f32 = 3.;
/// Tops of the two file rows under the folder; the second is the shown file.
const FIRST_FILE_TOP: f32 = 4.5;
const SHOWN_FILE_TOP: f32 = 9.;

/// What a click on the button runs.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Shows a file in the file tree; its own click does not reach the row.
#[derive(IntoElement)]
pub struct TreeLocateButton {
    id: ElementId,
    on_click: ClickHandler,
}

impl TreeLocateButton {
    pub fn new(
        id: impl Into<ElementId>,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            on_click: Box::new(on_click),
        }
    }
}

impl RenderOnce for TreeLocateButton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // A bar at `left`, `top` of the glyph, `width` × `height`, in `color`.
        let bar = |left: f32, top: f32, width: f32, height: f32, color| {
            div()
                .absolute()
                .left(px(left))
                .top(px(top))
                .w(px(width))
                .h(px(height))
                .rounded(px(1.))
                .bg(color)
        };
        // A branch from the rail to a file row whose top is `top`.
        let branch = |top: f32| {
            bar(
                RAIL_LEFT,
                top + FILE_HEIGHT / 2. - 0.5,
                FILE_LEFT - RAIL_LEFT - 1.,
                1.,
                theme::muted(),
            )
        };

        // The folder, the rail below it, a file row and the shown file's row.
        let glyph = div()
            .relative()
            .w(px(GLYPH_WIDTH))
            .h(px(GLYPH_HEIGHT))
            .child(bar(0., 0., FOLDER_WIDTH, FOLDER_HEIGHT, theme::muted()))
            .child(bar(
                RAIL_LEFT,
                FOLDER_HEIGHT,
                1.,
                SHOWN_FILE_TOP + FILE_HEIGHT / 2. - FOLDER_HEIGHT,
                theme::muted(),
            ))
            .child(branch(FIRST_FILE_TOP))
            .child(bar(
                FILE_LEFT,
                FIRST_FILE_TOP,
                GLYPH_WIDTH - FILE_LEFT,
                FILE_HEIGHT,
                theme::muted(),
            ))
            .child(branch(SHOWN_FILE_TOP))
            .child(bar(
                FILE_LEFT,
                SHOWN_FILE_TOP,
                GLYPH_WIDTH - FILE_LEFT,
                FILE_HEIGHT,
                theme::accent(),
            ));

        // The clickable square around the glyph.
        let on_click = self.on_click;
        div()
            .id(self.id)
            .flex_none()
            .size(px(BUTTON_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .hover(|style| style.bg(theme::hover()))
            .on_click(move |event, window, cx| {
                cx.stop_propagation();
                on_click(event, window, cx);
            })
            .child(glyph)
    }
}
