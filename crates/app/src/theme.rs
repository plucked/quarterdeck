//! The app's dark palette and fonts. Views take every color and font from
//! here, so a theme change touches only this file.

use gpui::{Rgba, rgb};

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
/// Error messages.
pub fn error() -> Rgba {
    rgb(0xf0_71_78)
}
