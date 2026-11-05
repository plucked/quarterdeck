use gpui::{Div, FontWeight, Rgba, div, prelude::*, px};

use crate::theme;

/// Edge length of the badge.
pub const FILE_STATUS_BADGE_SIZE: f32 = 15.;

/// A one-letter badge for GitHub's file status, colored like GitHub's:
/// A added, D removed, R renamed or moved, C copied, M modified.
pub fn render_file_status_badge(status: &str) -> Div {
    let (letter, color): (&str, Rgba) = match status {
        "added" => ("A", theme::added()),
        "removed" => ("D", theme::removed()),
        "renamed" => ("R", theme::renamed()),
        "copied" => ("C", theme::renamed()),
        _ => ("M", theme::accent()),
    };
    div()
        .flex_none()
        .w(px(FILE_STATUS_BADGE_SIZE))
        .h(px(FILE_STATUS_BADGE_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(3.))
        .border_1()
        .border_color(color)
        .text_size(px(9.))
        .font_weight(FontWeight::BOLD)
        .text_color(color)
        .child(letter)
}
