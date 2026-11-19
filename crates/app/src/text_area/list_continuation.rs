use std::ops::Range;

use gpui::Context;
use qd_markdown::{InlineMark, InlineStyle, RichDocument, container_continuation};

use super::TextArea;
use super::markdown_format::code_delimiter;
use super::text_boundaries::{line_end, line_start};

/// `CommonMark` recognizes up to nine digits in an ordered list marker.
const LARGEST_LIST_NUMBER: u32 = 999_999_999;

/// One source replacement and the caret's byte offset within it.
struct ListEdit {
    source: Range<usize>,
    replacement: String,
    caret: usize,
}

impl TextArea {
    /// Performs a list split as one editing command so undo restores the original selection.
    pub(super) fn continue_rich_list(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(rich) = &self.markdown.rich else {
            return false;
        };
        // Source mode, code fences and selections across paragraphs use ordinary Return.
        let selection = self.marked_range.as_ref().unwrap_or(&self.selected_range);
        let text = self.text();
        let row = line_start(text, selection.start);
        let end = line_end(text, selection.start);
        if selection.end > end || rich.blocks.enclosing_fence(selection).is_some() {
            return false;
        }
        let Some(marker) = rich.blocks.list_item_at(selection.start) else {
            return false;
        };
        if selection.start < marker.end {
            return false;
        }

        // Retain quote and indentation levels. An outer list marker on the same line
        // becomes padding so the next nested item remains under the same parent.
        let marker_row = line_start(text, marker.start);
        let container = container_continuation(&text[marker_row..marker.start]);
        let edit = if row == marker_row && text[marker.end..end].trim().is_empty() {
            exit_list(text, row..end, container)
        } else {
            let next = next_marker(&text[marker.clone()]);
            split_item(rich, text, selection, &format!("\n{container}{next}"))
        };

        // Retain this precise caret when undo and redo replay the split.
        let caret = edit.source.start + edit.caret;
        self.markdown.pending.clear();
        self.replace(edit.source, &edit.replacement, cx);
        self.selected_range = caret..caret;
        self.finish_edit_selection();
        true
    }
}

/// A blank line separates the next paragraph from a lazy continuation of the previous item.
fn exit_list(text: &str, row: Range<usize>, container: String) -> ListEdit {
    let previous = text[..row.start]
        .strip_suffix('\n')
        .unwrap_or("")
        .rsplit('\n')
        .next()
        .unwrap_or("");
    let replacement = if previous.trim().is_empty() {
        container
    } else {
        format!("{container}\n{container}")
    };
    ListEdit {
        source: row,
        caret: replacement.len(),
        replacement,
    }
}

/// Advances a parsed ordered marker and opens a fresh task while retaining the writer's list syntax.
fn next_marker(marker: &str) -> String {
    let digits = marker.bytes().take_while(u8::is_ascii_digit).count();
    let mut next = String::new();
    if digits > 0 {
        let next_number = match marker[..digits].parse::<u32>() {
            Ok(number) if number < LARGEST_LIST_NUMBER => number + 1,
            _ => 1,
        };
        next.push_str(&next_number.to_string());
    }
    next.push_str(&marker[digits..].replace("[x]", "[ ]").replace("[X]", "[ ]"));
    if !next.ends_with([' ', '\t']) {
        next.push(' ');
    }
    next
}

/// Close each inline style on the old item and reopen it on the new one, preserving link targets.
fn split_item(
    rich: &RichDocument,
    text: &str,
    selection: &Range<usize>,
    separator: &str,
) -> ListEdit {
    // Markdown emphasis cannot open or close against whitespace. Keep adjacent
    // spaces outside the split delimiters and the caret before any following spaces.
    let left = text[..selection.start].trim_end_matches([' ', '\t']).len();
    let right = text.len() - text[selection.end..].trim_start_matches([' ', '\t']).len();
    let left = if rich
        .marks
        .iter()
        .any(|mark| mark.inner.start < left && selection.start <= mark.inner.end)
    {
        left
    } else {
        selection.start
    };
    let right = if rich
        .marks
        .iter()
        .any(|mark| mark.inner.start <= selection.end && right < mark.inner.end)
    {
        right
    } else {
        selection.end
    };
    let mut cut = left..right;
    let displayed = rich.visible_range(&cut);

    // Close the innermost formatting first. Drop empty spans on the old item.
    let mut replacement = String::new();
    for mark in rich
        .marks
        .iter()
        .rev()
        .filter(|mark| mark.outer.start < left && left < mark.outer.end)
    {
        if displayed.start <= rich.visible(mark.inner.start) {
            cut.start = cut.start.min(mark.outer.start);
        } else if mark.style == InlineStyle::CODE {
            let content = &text[mark.inner.start..left.min(mark.inner.end)];
            cut.start = mark.outer.start;
            replacement.push_str(&code_fragment(content, mark, text).0);
        } else {
            replacement.push_str(&text[mark.inner.end..mark.outer.end]);
        }
    }
    replacement.push_str(&text[left..selection.start]);
    replacement.push_str(separator);
    let before_spaces = replacement.len();
    replacement.push_str(&text[selection.end..right]);
    let mut right_content_len = 0;

    // Reopen remaining spans from the outside in, including their original link destinations.
    for mark in rich
        .marks
        .iter()
        .filter(|mark| mark.outer.start < right && right < mark.outer.end)
    {
        if displayed.end >= rich.visible(mark.inner.end) {
            cut.end = cut.end.max(mark.outer.end);
        } else if mark.style == InlineStyle::CODE {
            let content = &text[right.max(mark.inner.start)..mark.inner.end];
            let (fragment, opening_len) = code_fragment(content, mark, text);
            cut.end = mark.outer.end;
            right_content_len = fragment.len() - opening_len;
            replacement.push_str(&fragment);
        } else {
            replacement.push_str(&text[mark.outer.start..mark.inner.start]);
        }
    }
    let caret = if right > selection.end {
        before_spaces
    } else {
        replacement.len() - right_content_len
    };
    ListEdit {
        source: cut,
        replacement,
        caret,
    }
}

/// Encode each surviving code span independently. Splitting a literal backtick run
/// can change both its required delimiter width and its need for surrounding spaces.
fn code_fragment(content: &str, mark: &InlineMark, text: &str) -> (String, usize) {
    let previous_width = text[mark.outer.start..mark.inner.start]
        .bytes()
        .take_while(|byte| *byte == b'`')
        .count();
    let ticks = code_delimiter(content, previous_width);
    let padding = if content.starts_with('`')
        || content.ends_with('`')
        || (content.starts_with(' ') && content.ends_with(' ') && !content.trim().is_empty())
    {
        " "
    } else {
        ""
    };
    (
        format!("{ticks}{padding}{content}{padding}{ticks}"),
        ticks.len() + padding.len(),
    )
}
