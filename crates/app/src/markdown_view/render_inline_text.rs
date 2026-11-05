use gpui::{
    AnyElement, FontStyle, FontWeight, HighlightStyle, InteractiveText, IntoElement,
    StrikethroughStyle, StyledText, UnderlineStyle, px,
};
use qc_markdown::{InlineStyle, InlineText};

use crate::theme;

/// URL prefixes a click may open; anything else (`file:`, app schemes,
/// relative paths) stays inert, since descriptions come from other people.
const OPENABLE_URL_PREFIXES: [&str; 2] = ["https://", "http://"];

/// Renders inline text with its styles; when it has links, clicking one
/// opens it in the browser. `id` must be unique within the enclosing
/// `MarkdownView`.
pub fn render_inline_text(text: &InlineText, id: usize) -> AnyElement {
    let highlights = text
        .runs()
        .iter()
        .map(|run| (run.range.clone(), highlight_for(run.style)));
    let styled = StyledText::new(text.text().to_owned()).with_highlights(highlights);
    if text.links().is_empty() {
        return styled.into_any_element();
    }

    // Clicks report the index of the range hit; the URLs share that order.
    let ranges = text.links().iter().map(|link| link.range.clone()).collect();
    let urls: Vec<String> = text.links().iter().map(|link| link.url.clone()).collect();
    InteractiveText::new(id, styled)
        .on_click(ranges, move |index, _window, cx| {
            if let Some(url) = urls.get(index).filter(|url| is_openable(url)) {
                cx.open_url(url);
            }
        })
        .into_any_element()
}

/// The text highlight for a combination of Markdown styles.
fn highlight_for(style: InlineStyle) -> HighlightStyle {
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
    highlight
}

/// Whether a link may be opened: absolute web URLs only.
fn is_openable(url: &str) -> bool {
    OPENABLE_URL_PREFIXES.iter().any(|prefix| {
        url.get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Web links open, in any letter case.
    #[test]
    fn web_links_are_openable() {
        assert!(is_openable("https://github.com/a/b"));
        assert!(is_openable("HTTP://example.com"));
    }

    /// Local files, other schemes and relative paths do not open.
    #[test]
    fn other_links_are_inert() {
        assert!(!is_openable("file:///etc/passwd"));
        assert!(!is_openable("vscode://open?x"));
        assert!(!is_openable("docs/readme.md"));
    }
}
