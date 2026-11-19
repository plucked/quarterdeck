use gpui::Context;

use super::TextArea;
use super::text_boundaries::line_start;

/// The GitHub Markdown that a toolbar button applies to the current selection.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MarkdownFormat {
    Heading,
    Bold,
    Italic,
    Strike,
    Code,
    CodeBlock,
    Link,
    Quote,
    Bullets,
    Numbered,
    Checklist,
}

impl MarkdownFormat {
    /// Priority order for the toolbar when only some controls fit beside the draft.
    pub(super) const TOOLBAR: [Self; 11] = [
        Self::Bold,
        Self::Italic,
        Self::Code,
        Self::Link,
        Self::Heading,
        Self::Strike,
        Self::CodeBlock,
        Self::Quote,
        Self::Bullets,
        Self::Numbered,
        Self::Checklist,
    ];

    pub(super) fn icon(self) -> &'static str {
        match self {
            Self::Bold => "icons/format-bold.svg",
            Self::Italic => "icons/format-italic.svg",
            Self::Code => "icons/format-code.svg",
            Self::Link => "icons/format-link.svg",
            Self::Heading => "icons/format-heading.svg",
            Self::Strike => "icons/format-strike.svg",
            Self::CodeBlock => "icons/format-block.svg",
            Self::Quote => "icons/format-quote.svg",
            Self::Bullets => "icons/format-bullets.svg",
            Self::Numbered => "icons/format-numbered.svg",
            Self::Checklist => "icons/format-checklist.svg",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Heading => "Heading",
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Strike => "Strikethrough",
            Self::Code => "Inline code",
            Self::CodeBlock => "Code block",
            Self::Link => "Link",
            Self::Quote => "Quote",
            Self::Bullets => "Bulleted list",
            Self::Numbered => "Numbered list",
            Self::Checklist => "Checklist",
        }
    }

    /// Inserts Markdown with the editor's normal change notification, preserving
    /// the selection inside the markup or leaving the caret ready for typing.
    pub(super) fn apply(self, area: &mut TextArea, cx: &mut Context<TextArea>) {
        if !area.allows_rich_format(self)
            || area.toggle_rich_block(self, cx)
            || area.handle_rich_format(self, cx)
        {
            return;
        }
        match self {
            Self::Bold => mark_words(area, "**", cx),
            Self::Italic => mark_words(area, "*", cx),
            Self::Strike => mark_words(area, "~~", cx),
            Self::Code => {
                let range = area.wrapping_selection();
                let selected = &area.text()[range.clone()];
                let delimiter = code_delimiter(selected, 1);
                let padding = if selected.starts_with('`') || selected.ends_with('`') {
                    " "
                } else {
                    ""
                };
                wrap_range(
                    area,
                    range,
                    &format!("{delimiter}{padding}"),
                    &format!("{padding}{delimiter}"),
                    cx,
                );
            }
            Self::CodeBlock => {
                let range = area.selected_range.clone();
                let text = area.text();
                let fence = code_delimiter(&text[range.clone()], 3);
                // A fence occupies its own line even when the selection starts mid-sentence.
                let before = if range.start > 0 && !text[..range.start].ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                let after = if range.end < text.len() && !text[range.end..].starts_with('\n') {
                    "\n"
                } else {
                    ""
                };
                let closing_line = if text[range].ends_with('\n') {
                    ""
                } else {
                    "\n"
                };
                wrap_selection(
                    area,
                    &format!("{before}{fence}\n"),
                    &format!("{closing_line}{fence}{after}"),
                    cx,
                );
            }
            Self::Link => insert_link(area, cx),
            Self::Heading => prefix_lines(area, |_| "### ".into(), cx),
            Self::Quote => prefix_lines(area, |_| "> ".into(), cx),
            Self::Bullets => prefix_lines(area, |_| "- ".into(), cx),
            Self::Numbered => prefix_lines(area, |number| format!("{number}. "), cx),
            Self::Checklist => prefix_lines(area, |_| "- [ ] ".into(), cx),
        }
        area.finish_edit_selection();
    }
}

/// Creates a fence or inline delimiter longer than any backtick run in the selection.
pub(super) fn code_delimiter(selected: &str, minimum: usize) -> String {
    let longest = selected
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    "`".repeat(minimum.max(longest + 1))
}

/// Emphasis and strikethrough markers must touch the words they format.
fn mark_words(area: &mut TextArea, marker: &str, cx: &mut Context<TextArea>) {
    let mut range = area.wrapping_selection();
    let selected = &area.text()[range.clone()];
    let words = selected.trim();
    if !words.is_empty() {
        let start = range.start + (selected.len() - selected.trim_start().len());
        range = start..start + words.len();
    }
    wrap_range(area, range, marker, marker, cx);
}

/// Wraps the selected bytes and keeps just those bytes selected between the markers.
fn wrap_selection(area: &mut TextArea, before: &str, after: &str, cx: &mut Context<TextArea>) {
    wrap_range(area, area.selected_range.clone(), before, after, cx);
}

/// Formats the requested range while preserving the original selection for Undo.
fn wrap_range(
    area: &mut TextArea,
    range: std::ops::Range<usize>,
    before: &str,
    after: &str,
    cx: &mut Context<TextArea>,
) {
    let selected = &area.text()[range.clone()];
    let replacement = format!("{before}{selected}{after}");
    let content = range.start + before.len()..range.end + before.len();
    area.replace(range, &replacement, cx);
    area.selected_range = content;
}

/// Makes selected words the link label and selects its URL for the next keystroke.
/// At a caret the label is the first editable placeholder.
fn insert_link(area: &mut TextArea, cx: &mut Context<TextArea>) {
    let range = area.selected_range.clone();
    let url = "url";
    let label = if range.is_empty() {
        "text"
    } else {
        &area.text()[range.clone()]
    };
    let label_start = range.start + "[".len();
    let url_start = label_start + label.len() + "](".len();
    let selection = if range.is_empty() {
        label_start..label_start + label.len()
    } else {
        url_start..url_start + url.len()
    };
    area.replace(range, &format!("[{label}]({url})"), cx);
    area.selected_range = selection;
}

/// Adds a prefix at the beginning of every touched logical line, keeping a caret
/// at its original character and a selection over the same passage.
fn prefix_lines(area: &mut TextArea, prefix: impl Fn(usize) -> String, cx: &mut Context<TextArea>) {
    let range = area.selected_range.clone();
    let start = line_start(area.text(), range.start);
    let lines = &area.text()[start..range.end];
    let first_prefix = prefix(1);
    let selection_start = range.start + first_prefix.len();
    let mut replacement = first_prefix;
    for (index, line) in lines.split_inclusive('\n').enumerate() {
        if index > 0 {
            replacement.push_str(&prefix(index + 1));
        }
        replacement.push_str(line);
    }
    let inserted = area.replace(start..range.end, &replacement, cx);
    area.selected_range = selection_start..inserted.end;
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    /// Formatting retains the selected UTF-8 text and inserts an editable pair at a bare caret.
    #[gpui::test]
    fn inline_format_preserves_selected_content(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::new("", cx));
        editor.update(cx, |area, cx| {
            area.set_text("a café!", cx);
            area.selected_range = 2..7;
            MarkdownFormat::Bold.apply(area, cx);
            assert_eq!(area.text(), "a **café**!");
            assert_eq!(&area.text()[area.selected_range.clone()], "café");

            area.set_text("  café\u{a0}\n", cx);
            area.selected_range = 0..area.text().len();
            MarkdownFormat::Strike.apply(area, cx);
            assert_eq!(area.text(), "  ~~café~~\u{a0}\n");
            assert_eq!(&area.text()[area.selected_range.clone()], "café");

            area.set_text("π", cx);
            MarkdownFormat::Italic.apply(area, cx);
            assert_eq!(area.text(), "π**");
            assert_eq!(area.selected_range, 3..3);
        });
    }

    /// Link insertion selects the URL for selected words and the label for an empty selection.
    #[gpui::test]
    fn links_offer_the_next_editable_part(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::new("", cx));
        editor.update(cx, |area, cx| {
            area.set_text("see café", cx);
            area.selected_range = 4..9;
            MarkdownFormat::Link.apply(area, cx);
            assert_eq!(area.text(), "see [café](url)");
            assert_eq!(&area.text()[area.selected_range.clone()], "url");

            area.set_text("", cx);
            MarkdownFormat::Link.apply(area, cx);
            assert_eq!(area.text(), "[text](url)");
            assert_eq!(&area.text()[area.selected_range.clone()], "text");
        });
    }

    /// A list formats each touched line and leaves a line starting exactly at the selection end alone.
    #[gpui::test]
    fn lists_prefix_complete_lines(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::new("", cx));
        editor.update(cx, |area, cx| {
            area.set_text("αx\nβ\nnext", cx);
            area.selected_range = 2..7;
            MarkdownFormat::Numbered.apply(area, cx);
            assert_eq!(area.text(), "1. αx\n2. β\nnext");
            assert_eq!(&area.text()[area.selected_range.clone()], "x\n2. β\n");

            area.set_text("first\n", cx);
            MarkdownFormat::Checklist.apply(area, cx);
            assert_eq!(area.text(), "first\n- [ ] ");
            assert_eq!(area.cursor_offset(), area.text().len());
        });
    }

    /// Code containing backticks stays inside the chosen delimiter, including mid-line block insertions.
    #[gpui::test]
    fn code_uses_matching_delimiters(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::new("", cx));
        editor.update(cx, |area, cx| {
            area.set_text("`key`", cx);
            area.selected_range = 0..area.text().len();
            MarkdownFormat::Code.apply(area, cx);
            assert_eq!(area.text(), "`` `key` ``");

            area.set_text("x```y", cx);
            area.selected_range = 1..4;
            MarkdownFormat::CodeBlock.apply(area, cx);
            assert_eq!(area.text(), "x\n````\n```\n````\ny");
            assert_eq!(&area.text()[area.selected_range.clone()], "```");
        });
    }
}
