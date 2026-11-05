//! The app's palette and fonts: Graphite, a dark theme built for reading
//! code. A neutral dark grey ground (not black) with off-white text
//! (13:1, APCA Lc −88), secondary text at 4.5:1 or more, and diffs in blue
//! (added) and orange (removed), which stay apart for red-green colour
//! blindness; green and red only mark success and danger. Views take every
//! color and font from here, so a theme change touches only this file.

use gpui::{Rgba, rgb, rgba};

/// Font for the interface: `Schibsted Grotesk`, built into the app
/// (`bundled_fonts`).
pub const UI_FONT: &str = "Schibsted Grotesk";
/// Font for code and patches: `JetBrains Mono`, built into the app; its
/// tall lower case and distinct `l`, `1`, `I`, `0`, `O` suit reading diffs.
pub const MONO_FONT: &str = "JetBrains Mono";

/// The diff's canvas, behind the file cards.
pub fn background() -> Rgba {
    rgb(0x1c_1e_21)
}
/// The queue sidebar: a step darker than the content.
pub fn sidebar() -> Rgba {
    rgb(0x17_19_1c)
}
/// Headers and the side panel.
pub fn panel() -> Rgba {
    rgb(0x20_22_26)
}
/// Cards and controls raised above a panel: file headers, chips, buttons.
pub fn surface() -> Rgba {
    rgb(0x26_29_2e)
}
/// Quiet dividers and card edges.
pub fn border() -> Rgba {
    rgb(0x30_32_35)
}
/// Edges that must stand out: inputs, outlined buttons.
pub fn border_strong() -> Rgba {
    rgb(0x40_43_47)
}
/// A hovered row or control.
pub fn hover() -> Rgba {
    rgb(0x2a_2d_33)
}
/// The selected row, chip or segment.
pub fn selected() -> Rgba {
    rgb(0x24_3b_52)
}
pub fn text() -> Rgba {
    rgb(0xe5_e3_df)
}
/// Secondary text: metadata, placeholders, hunk headers.
pub fn muted() -> Rgba {
    rgb(0x9d_99_8e)
}
/// Tertiary text: hints, counts, separators.
pub fn subtle() -> Rgba {
    rgb(0x8b_88_7f)
}
/// Links, primary buttons, focus and progress.
pub fn accent() -> Rgba {
    rgb(0x95_cb_fe)
}
/// Text on an accent background.
pub fn on_accent() -> Rgba {
    rgb(0x17_19_1c)
}
/// The scrollbar thumb at rest.
pub fn scrollbar_thumb() -> Rgba {
    rgba(0xe5_e3_df_33)
}
/// The scrollbar thumb while dragged.
pub fn scrollbar_thumb_active() -> Rgba {
    rgba(0xe5_e3_df_66)
}
/// Background of selected text.
pub fn text_selection() -> Rgba {
    rgba(0x3b_6a_9c_99)
}
/// Tint over the diff lines a comment being written is about.
pub fn comment_range() -> Rgba {
    rgba(0x95_cb_fe_1f)
}
/// Background of code blocks and inline code in rendered Markdown.
pub fn code_background() -> Rgba {
    rgb(0x17_19_1c)
}
/// Something that worked or is ready: an open pull request, a done review.
pub fn success() -> Rgba {
    rgb(0x8c_e1_99)
}
/// Destructive actions and blockers: delete, close, unmet requirements.
pub fn danger() -> Rgba {
    rgb(0xfe_bf_b7)
}
/// Added lines: counts, the added badge, the gutter mark (blue).
pub fn added() -> Rgba {
    rgb(0x95_cb_fe)
}
/// Removed lines: counts, the removed badge, the gutter mark (orange).
pub fn removed() -> Rgba {
    rgb(0xff_b5_89)
}
/// Background of an added patch line.
pub fn added_background() -> Rgba {
    rgb(0x18_28_37)
}
/// Background of a removed patch line.
pub fn removed_background() -> Rgba {
    rgb(0x34_22_17)
}
/// Background of the changed words inside an added line.
pub fn added_emphasis() -> Rgba {
    rgb(0x14_39_59)
}
/// Background of the changed words inside a removed line.
pub fn removed_emphasis() -> Rgba {
    rgb(0x52_2b_10)
}
/// Background of a hunk header row and of the empty side of a split row.
pub fn diff_filler() -> Rgba {
    rgb(0x19_1b_1e)
}

/// Muted hues that tell repositories apart (each repository keeps one,
/// picked from its name), dark enough for light text on them.
pub fn label_colors() -> [Rgba; 8] {
    [
        rgb(0x3f_6f_c4),
        rgb(0x7d_5b_c2),
        rgb(0x2f_8a_80),
        rgb(0xb0_6a_30),
        rgb(0xb0_4f_83),
        rgb(0x4f_8f_47),
        rgb(0x9a_84_2c),
        rgb(0xb0_51_51),
    ]
}

/// Background of a hunk's `@@ … @@` row.
pub fn hunk_background() -> Rgba {
    rgb(0x17_19_1c)
}
/// File status badge for a renamed file.
pub fn renamed() -> Rgba {
    rgb(0xde_c3_ff)
}
/// Error messages.
pub fn error() -> Rgba {
    rgb(0xfe_bf_b7)
}
