use std::ops::Range;

use gpui::Context;
use qd_markdown::{FenceDelimiters, MarkdownFence};

use super::TextArea;
use super::comment_history::EditSource;
use super::markdown_format::code_delimiter;

impl TextArea {
    /// A suggestion targets exactly the rows it was written against, even across Undo and Redo.
    pub(crate) fn suggestion_anchors_range(&self) -> bool {
        self.markdown.suggestion_in_history
    }

    /// Adds a suggested replacement after any existing explanation and selects
    /// its code so typing immediately edits the proposal.
    pub(crate) fn insert_suggestion(&mut self, source: &str, cx: &mut Context<Self>) {
        if let Some(existing) = self.markdown.suggestion.clone() {
            self.markdown.preview = None;
            self.selected_range = existing.code;
            self.selection_reversed = false;
            self.sync_rich_selection(cx);
            cx.emit(super::TextAreaEvent::LayoutChanged);
            cx.notify();
            return;
        }
        let body = self.text();
        let separator = if body.is_empty() || body.ends_with("\n\n") {
            ""
        } else if body.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        let fence = code_delimiter(source, 3);
        let opening = format!("{fence}suggestion\n");
        let block = format!("{opening}{source}\n{fence}");
        let end = body.len();
        let appended = format!("{separator}{block}");
        // A draft may end inside an unfinished code example or HTML comment.
        // In that case place the proposal before it, where GitHub can recognize it.
        let (at, inserted, selection_start) =
            if MarkdownFence::suggestion_in(&format!("{body}{appended}")).is_some() {
                (end, appended, end + separator.len() + opening.len())
            } else {
                (0, format!("{block}\n\n"), opening.len())
            };
        self.replace(at..at, &inserted, cx);
        self.selected_range = selection_start..selection_start + source.len();
        self.finish_edit_selection();
        self.sync_rich_selection(cx);
    }

    /// Edit literal payload as plain text, retaining the original fence, language,
    /// whitespace and line endings. Changes to the delimiters share the same undo step.
    pub(super) fn edit_fenced_code(
        &mut self,
        range: &Range<usize>,
        text: &str,
        source: EditSource,
        cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let block = self.markdown.rich.as_ref()?.blocks.enclosing_fence(range)?;
        if range.start < block.code.start || range.end > block.code.end {
            return None;
        }
        let body = self.text();
        let encoded = block.source_for_input(body, range, text);
        let text = encoded.as_ref();
        let marker = char::from(body.as_bytes()[block.opening.start]);
        // Removing a suffix or splitting a line can turn an existing marker run into a closer.
        let may_expose_marker = !range.is_empty() || text.contains([marker, '\n']);
        let header = &body[block.opening.end..block.code.start];
        let needs_header_break = !text.is_empty() && !header.contains('\n');
        let needs_closing_break = !text.is_empty()
            && block
                .closing
                .as_ref()
                .is_some_and(|closing| !body[block.code.end..closing.start].contains('\n'));
        if !may_expose_marker && !needs_header_break && !needs_closing_break {
            return Some(self.replace_from(range.clone(), text, source, cx));
        }
        let mut code = body[block.code.clone()].to_owned();
        code.replace_range(
            range.start - block.code.start..range.end - block.code.start,
            text,
        );
        let delimiters = FenceDelimiters::for_code(block, body, &code);
        if delimiters.opening == body[block.outer.start..block.code.start]
            && delimiters.closing == body[block.code.end..block.outer.end]
        {
            return Some(self.replace_from(range.clone(), text, source, cx));
        }
        let start = block.outer.start + delimiters.opening.len() + range.start - block.code.start;
        let replacement = format!("{}{code}{}", delimiters.opening, delimiters.closing);
        self.replace_from(block.outer.clone(), &replacement, source, cx);
        let inserted = start..start + text.len();
        self.selected_range = inserted.end..inserted.end;
        self.finish_edit_selection();
        Some(inserted)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    /// Original code is editable in place, even when it contains a Markdown fence or ends in a blank line.
    #[gpui::test]
    fn suggestion_preserves_prose_and_selects_only_code(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::comment("", cx));
        editor.update(cx, |area, cx| {
            area.set_text("Try this.", cx);
            area.selected_range = 0..3;
            let source = "\tλ  \n```\n";
            area.insert_suggestion(source, cx);
            assert_eq!(
                area.text(),
                "Try this.\n\n````suggestion\n\tλ  \n```\n\n````"
            );
            assert_eq!(&area.text()[area.selected_range.clone()], source);

            area.replace(area.selected_range.clone(), "changed", cx);
            assert_eq!(area.text(), "Try this.\n\n````suggestion\nchanged\n````");
            area.insert_suggestion(source, cx);
            assert_eq!(area.text(), "Try this.\n\n````suggestion\nchanged\n````");
            assert_eq!(&area.text()[area.selected_range.clone()], "changed");
            assert!(area.suggestion_anchors_range());
        });
    }

    /// An empty head-side line leaves a caret between the fences for the reviewer to fill.
    #[gpui::test]
    fn blank_line_suggestion_has_an_editable_caret(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::comment("", cx));
        editor.update(cx, |area, cx| {
            area.insert_suggestion("", cx);
            area.replace(area.selected_range.clone(), "new", cx);
            assert_eq!(area.text(), "```suggestion\nnew\n```");
        });
    }

    /// An unfinished example cannot swallow a suggestion, and quoted examples do not lock a draft.
    #[gpui::test]
    fn suggestion_is_a_top_level_block(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::comment("", cx));
        editor.update(cx, |area, cx| {
            area.set_text(
                "> ```suggestion\n> quoted example\n> ```\n\n```rust\nexample",
                cx,
            );
            assert!(!area.suggestion_anchors_range());
            area.insert_suggestion("fixed", cx);
            assert!(area.text().starts_with("```suggestion\nfixed\n```\n\n>"));
        });
    }

    /// Reopening the suggestion selects short backticks and all trailing blank lines in either mode.
    #[gpui::test]
    fn selecting_an_unclosed_suggestion_uses_the_cached_payload(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::comment("", cx));
        editor.update(cx, |area, cx| {
            for github in [false, true] {
                area.set_github_markdown(github, cx);
                for code in ["value\n```", "value\n\n"] {
                    let body = format!("````suggestion\n{code}");
                    area.set_text(&body, cx);
                    assert!(area.suggestion_anchors_range());
                    area.insert_suggestion("unused", cx);
                    assert_eq!(area.text(), body);
                    assert_eq!(&body[area.selected_range.clone()], code);
                    area.set_text("> ````suggestion\n> quoted", cx);
                    assert!(!area.suggestion_anchors_range());
                }
            }
        });
    }
}
