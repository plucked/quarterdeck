//! `GitHubLink`: GitHub's mark as a small icon button opening a page on GitHub.

use gpui::{
    App, ElementId, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px, svg,
};

use crate::bundled_icons::GITHUB_MARK;
use crate::theme;

/// Size of the drawn mark.
const MARK_SIZE: f32 = 16.;
/// Size of the square the mark sits in, the button's hit area.
const BUTTON_SIZE: f32 = 24.;
/// The hover group tinting the mark: an `svg` takes only its own text color.
const HOVER_GROUP: &str = "github-link";

/// GitHub's mark in the muted text color (the text color on hover) that
/// opens `url` in the browser when clicked; the click goes no further, so
/// a clickable row around it ignores it.
#[derive(IntoElement)]
pub struct GitHubLink {
    id: ElementId,
    url: SharedString,
}

impl GitHubLink {
    /// A link to `url`; `id` must be unique among its siblings.
    pub fn new(id: impl Into<ElementId>, url: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            url: url.into(),
        }
    }
}

impl RenderOnce for GitHubLink {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let url = self.url;
        div()
            .id(self.id)
            .flex_none()
            .size(px(BUTTON_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .group(HOVER_GROUP)
            .hover(|style| style.bg(theme::hover()))
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                cx.open_url(&url);
            })
            .child(
                svg()
                    .path(GITHUB_MARK)
                    .size(px(MARK_SIZE))
                    .text_color(theme::muted())
                    .group_hover(HOVER_GROUP, |style| style.text_color(theme::text())),
            )
    }
}
