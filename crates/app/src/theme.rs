//! The app's dark palette and fonts. Views take every color and font from
//! here, so a theme change touches only this file.

use gpui::{Rgba, rgb, rgba};

/// Font for code and patches.
pub const MONO_FONT: &str = "Menlo";

/// Window background.
pub fn background() -> Rgba {
    rgb(0x1e_1f_22)
}
/// Sidebar, headers and other raised surfaces.
pub fn panel() -> Rgba {
    rgb(0x2b_2d_30)
}
pub fn border() -> Rgba {
    rgb(0x3c_3f_41)
}
/// A hovered row.
pub fn hover() -> Rgba {
    rgb(0x35_37_3b)
}
/// The selected row.
pub fn selected() -> Rgba {
    rgb(0x2e_43_6e)
}
pub fn text() -> Rgba {
    rgb(0xdf_e1_e5)
}
/// Secondary text: metadata, placeholders, hunk headers.
pub fn muted() -> Rgba {
    rgb(0x8c_8f_94)
}
/// Links and buttons.
pub fn accent() -> Rgba {
    rgb(0x54_8a_f7)
}
/// The scrollbar thumb at rest.
pub fn scrollbar_thumb() -> Rgba {
    rgba(0xdf_e1_e5_33)
}
/// The scrollbar thumb while dragged.
pub fn scrollbar_thumb_active() -> Rgba {
    rgba(0xdf_e1_e5_66)
}
/// Background of selected text.
pub fn text_selection() -> Rgba {
    rgba(0x54_8a_f7_59)
}
/// Tint over the diff lines a comment being written is about.
pub fn comment_range() -> Rgba {
    rgba(0x54_8a_f7_2a)
}
/// Background of code blocks and inline code in rendered Markdown.
pub fn code_background() -> Rgba {
    rgb(0x1b_1c_1f)
}
/// Added-line count.
pub fn added() -> Rgba {
    rgb(0x5f_b8_65)
}
/// Removed-line count.
pub fn removed() -> Rgba {
    rgb(0xe0_6c_75)
}
/// Background of an added patch line.
pub fn added_background() -> Rgba {
    rgb(0x1f_33_24)
}
/// Background of a removed patch line.
pub fn removed_background() -> Rgba {
    rgb(0x3b_23_26)
}
/// Background of the changed words inside an added line.
pub fn added_emphasis() -> Rgba {
    rgb(0x2e_5a_37)
}
/// Background of the changed words inside a removed line.
pub fn removed_emphasis() -> Rgba {
    rgb(0x6b_32_38)
}
/// Background of a hunk header row and of the empty side of a split row.
pub fn diff_filler() -> Rgba {
    rgb(0x25_27_2b)
}
/// File status badge for a renamed file.
pub fn renamed() -> Rgba {
    rgb(0xd1_9a_66)
}
/// Error messages.
pub fn error() -> Rgba {
    rgb(0xf0_71_78)
}
