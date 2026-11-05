use gpui::{App, FontWeight, IntoElement, RenderOnce, SharedString, Window, div, prelude::*, px};

use crate::theme;

/// Edge length of the square.
const MONOGRAM_SIZE: f32 = 18.;

/// A repository's mark in the sidebar: the first letter of its name on a
/// rounded square in a color of its own, so repositories are told apart at
/// a glance. The color comes from the name, so it stays the same.
#[derive(IntoElement)]
pub struct RepoMonogram {
    name: SharedString,
}

impl RepoMonogram {
    pub fn new(name: impl Into<SharedString>) -> Self {
        Self { name: name.into() }
    }
}

impl RenderOnce for RepoMonogram {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let letter = self
            .name
            .chars()
            .find(char::is_ascii_alphanumeric)
            .map_or_else(String::new, |letter| {
                letter.to_ascii_uppercase().to_string()
            });
        div()
            .flex_none()
            .size(px(MONOGRAM_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.))
            .bg(color_of(&self.name))
            .text_size(px(10.))
            .font_weight(FontWeight::BOLD)
            .text_color(theme::on_accent())
            .child(letter)
    }
}

/// The color `name` always gets: its bytes hashed (FNV-1a) into the palette.
fn color_of(name: &str) -> gpui::Rgba {
    let colors = theme::label_colors();
    let hash = name.bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    let index = usize::try_from(hash).unwrap_or(0) % colors.len();
    colors[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name always gets the same color, and different names spread out.
    #[test]
    fn colors_are_stable_per_name() {
        assert_eq!(color_of("seed"), color_of("seed"));
        let distinct: std::collections::HashSet<String> =
            ["seed", "iac-gcloud", "library-eco", "homebrew-gnosis"]
                .into_iter()
                .map(|name| format!("{:?}", color_of(name)))
                .collect();
        assert!(distinct.len() > 1);
    }
}
