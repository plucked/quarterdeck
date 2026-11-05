//! The app's dark palette and fonts. Views take every color and font from
//! here, so a theme change touches only this file.

use gpui::{Rgba, rgb, rgba};

/// Font for the interface: the system font.
pub const UI_FONT: &str = ".SystemUIFont";
/// Font for code and patches.
pub const MONO_FONT: &str = "Menlo";

/// The diff's canvas, behind the file cards.
pub fn background() -> Rgba {
    rgb(0x1b_1c_1f)
}
/// The queue sidebar: a step darker than the content.
pub fn sidebar() -> Rgba {
    rgb(0x17_18_1a)
}
/// Headers and the side panel.
pub fn panel() -> Rgba {
    rgb(0x20_21_24)
}
/// Cards and controls raised above a panel: file headers, chips, buttons.
pub fn surface() -> Rgba {
    rgb(0x27_29_2d)
}
/// Quiet dividers and card edges.
pub fn border() -> Rgba {
    rgb(0x2e_30_35)
}
/// Edges that must stand out: inputs, outlined buttons.
pub fn border_strong() -> Rgba {
    rgb(0x3d_40_46)
}
/// A hovered row or control.
pub fn hover() -> Rgba {
    rgb(0x2c_2e_33)
}
/// The selected row, chip or segment.
pub fn selected() -> Rgba {
    rgb(0x26_35_52)
}
pub fn text() -> Rgba {
    rgb(0xe6_e7_ea)
}
/// Secondary text: metadata, placeholders, hunk headers.
pub fn muted() -> Rgba {
    rgb(0x9a_9d_a4)
}
/// Tertiary text: hints, counts, separators.
pub fn subtle() -> Rgba {
    rgb(0x6a_6e_76)
}
/// Links, primary buttons, focus and progress.
pub fn accent() -> Rgba {
    rgb(0x5b_8d_ef)
}
/// Text on an accent background.
pub fn on_accent() -> Rgba {
    rgb(0xff_ff_ff)
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
    rgba(0x5b_8d_ef_59)
}
/// Tint over the diff lines a comment being written is about.
pub fn comment_range() -> Rgba {
    rgba(0x5b_8d_ef_2a)
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
    rgb(0x22_24_28)
}

/// Background of a hunk's `@@ … @@` row.
pub fn hunk_background() -> Rgba {
    rgb(0x1e_24_31)
}
/// File status badge for a renamed file.
pub fn renamed() -> Rgba {
    rgb(0xd1_9a_66)
}
/// Error messages.
pub fn error() -> Rgba {
    rgb(0xf0_71_78)
}
