use gpui::{Rgba, rgb};
use qd_syntax::SyntaxKind;

use super::active_theme::active_colors;

/// One function per color of the active theme, named after its role.
macro_rules! color_tokens {
    ($($(#[$doc:meta])* $name:ident),* $(,)?) => {
        $(
            $(#[$doc])*
            pub fn $name() -> Rgba {
                active_colors().$name
            }
        )*
    };
}

color_tokens! {
    /// The diff's canvas, behind the file cards.
    background,
    /// The queue sidebar.
    sidebar,
    /// Headers and the side panel.
    panel,
    /// Cards and controls raised above a panel: file headers, chips, buttons.
    surface,
    /// Quiet dividers and card edges.
    border,
    /// Edges that must stand out: inputs, outlined buttons.
    border_strong,
    /// A hovered row or control.
    hover,
    /// The selected row, chip or segment.
    selected,
    text,
    /// Secondary text: metadata, placeholders (6:1 or more).
    muted,
    /// Tertiary text: hints, counts, separators, hunk headers (4.5:1 or more).
    subtle,
    /// Line numbers beside the diff.
    line_number,
    /// Links, primary buttons, focus and progress.
    accent,
    /// Text on an accent background.
    on_accent,
    /// The scrollbar thumb at rest.
    scrollbar_thumb,
    /// The scrollbar thumb while dragged.
    scrollbar_thumb_active,
    /// Background of selected text.
    text_selection,
    /// Tint over the diff lines a comment being written is about.
    comment_range,
    /// Background of code blocks and inline code in rendered Markdown.
    code_background,
    /// Something that worked or is ready: an open pull request, a done review.
    success,
    /// Destructive actions and blockers: delete, close, unmet requirements.
    danger,
    /// Added lines: counts, the added badge, the `+` mark (blue).
    added,
    /// Removed lines: counts, the removed badge, the `-` mark (orange).
    removed,
    /// Background of an added patch line.
    added_background,
    /// Background of a removed patch line.
    removed_background,
    /// Background of the changed words inside an added line.
    added_emphasis,
    /// Background of the changed words inside a removed line.
    removed_emphasis,
    /// Background of the empty side of a split row.
    diff_filler,
    /// Background of a hunk's `@@ … @@` row.
    hunk_background,
    /// File status badge for a renamed file; local notes.
    renamed,
    /// Error messages.
    error,
}

/// The color of a syntax token of `kind`.
pub fn syntax(kind: SyntaxKind) -> Rgba {
    active_colors().syntax(kind)
}

/// Text on a `label_colors` background, in every theme.
pub fn on_label() -> Rgba {
    rgb(0xff_ff_ff)
}

/// Muted hues that tell repositories apart (each repository keeps one,
/// picked from its name), dark enough for `on_label` text in every theme.
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
