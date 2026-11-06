use gpui::{App, IntoElement, Pixels, RenderOnce, SharedString, Window, div, img, prelude::*};

use super::AvatarImages;
use crate::theme;

/// A user's round avatar, `size` across: the image once [`AvatarImages`]
/// has it, a quiet circle until then. Its URL must have been requested.
#[derive(IntoElement)]
pub struct Avatar {
    url: SharedString,
    size: Pixels,
}

impl Avatar {
    pub fn new(url: impl Into<SharedString>, size: Pixels) -> Self {
        Self {
            url: url.into(),
            size,
        }
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let image = AvatarImages::global(cx).read(cx).image(&self.url);
        let circle = div()
            .flex_none()
            .size(self.size)
            .rounded_full()
            .overflow_hidden()
            .bg(theme::border());
        match image {
            Some(image) => circle.child(img(image).size(self.size).rounded_full()),
            None => circle,
        }
    }
}
