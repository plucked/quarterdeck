use gpui::{FontStyle, FontWeight, HighlightStyle, StrikethroughStyle, UnderlineStyle, px};
use qc_markdown::InlineStyle;

use crate::theme;

/// The text highlight for a piece of text: its Markdown styles, and the
/// selection background when selected (which covers a code background).
pub fn inline_highlight(style: InlineStyle, is_selected: bool) -> HighlightStyle {
    let mut highlight = HighlightStyle::default();
    if style.contains(InlineStyle::STRONG) {
        highlight.font_weight = Some(FontWeight::BOLD);
    }
    if style.contains(InlineStyle::EMPHASIS) {
        highlight.font_style = Some(FontStyle::Italic);
    }
    if style.contains(InlineStyle::STRIKETHROUGH) {
        highlight.strikethrough = Some(StrikethroughStyle {
            thickness: px(1.),
            color: None,
        });
    }
    if style.contains(InlineStyle::CODE) {
        highlight.background_color = Some(theme::code_background().into());
    }
    if style.contains(InlineStyle::LINK) {
        highlight.color = Some(theme::accent().into());
        highlight.underline = Some(UnderlineStyle {
            thickness: px(1.),
            color: None,
            wavy: false,
        });
    }
    if is_selected {
        highlight.background_color = Some(theme::text_selection().into());
    }
    highlight
}
