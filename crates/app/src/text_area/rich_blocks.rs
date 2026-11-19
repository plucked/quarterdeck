use std::ops::Range;

use gpui::Context;
use qd_markdown::BlockFormat;

use super::TextArea;
use super::markdown_format::MarkdownFormat;
use super::text_boundaries::line_start;

impl MarkdownFormat {
    fn block_format(self) -> Option<BlockFormat> {
        Some(match self {
            Self::Heading => BlockFormat::Heading,
            Self::Quote => BlockFormat::Quote,
            Self::Bullets => BlockFormat::Bullets,
            Self::Numbered => BlockFormat::Numbered,
            Self::Checklist => BlockFormat::Checklist,
            _ => return None,
        })
    }
}

fn is_list(format: MarkdownFormat) -> bool {
    matches!(
        format,
        MarkdownFormat::Bullets | MarkdownFormat::Numbered | MarkdownFormat::Checklist
    )
}

/// Source lines touched by the selection. A selection ending at column zero skips the following row.
fn touched_rows(text: &str, selection: &Range<usize>) -> impl Iterator<Item = usize> {
    let start = line_start(text, selection.start);
    std::iter::once(start).chain(
        text[start..selection.end]
            .match_indices('\n')
            .map(move |(offset, _)| start + offset + 1)
            .filter(move |offset| *offset < selection.end),
    )
}

/// Controls and clipboard metadata depend on whether this selection edits literal Markdown.
enum RichFormatContext {
    Prose,
    InlineCode,
    CodeBlock,
    ProtectedFence,
}

impl TextArea {
    fn rich_format_context(&self) -> RichFormatContext {
        let Some(rich) = self.markdown.rich.as_ref() else {
            return RichFormatContext::Prose;
        };
        let selection = &self.selected_range;
        if let Some(fence) = rich.blocks.fences().iter().find(|fence| {
            if selection.is_empty() {
                fence.code.start <= selection.start && selection.start <= fence.code.end
            } else {
                selection.start < fence.outer.end && fence.outer.start < selection.end
            }
        }) {
            if !fence.suggestion
                && fence.outer.start <= selection.start
                && selection.end <= fence.outer.end
            {
                return RichFormatContext::CodeBlock;
            }
            return RichFormatContext::ProtectedFence;
        }
        if rich.marks.iter().any(|mark| {
            mark.style == qd_markdown::InlineStyle::CODE
                && mark.inner.start <= selection.start
                && selection.end <= mark.inner.end
        }) {
            return RichFormatContext::InlineCode;
        }
        RichFormatContext::Prose
    }

    /// Rich clipboard metadata carries source markup. Inside code the clipboard's plain
    /// representation is the proposed text.
    pub(super) fn accepts_rich_paste(&self) -> bool {
        self.markdown.rich.is_some()
            && matches!(self.rich_format_context(), RichFormatContext::Prose)
    }

    /// The active code control can unwrap ordinary code. GitHub suggestions retain their fence.
    pub(super) fn allows_rich_format(&self, format: MarkdownFormat) -> bool {
        match self.rich_format_context() {
            RichFormatContext::Prose => true,
            RichFormatContext::InlineCode => format == MarkdownFormat::Code,
            RichFormatContext::CodeBlock => format == MarkdownFormat::CodeBlock,
            RichFormatContext::ProtectedFence => false,
        }
    }

    pub(super) fn rich_block_active(&self, format: MarkdownFormat) -> bool {
        let Some(rich) = &self.markdown.rich else {
            return false;
        };
        if format == MarkdownFormat::CodeBlock {
            return rich
                .blocks
                .enclosing_fence(&self.selected_range)
                .is_some_and(|fence| !fence.suggestion);
        }
        touched_rows(self.text(), &self.selected_range).all(|row| {
            format
                .block_format()
                .and_then(|kind| rich.blocks.prefix(row, kind))
                .is_some()
        })
    }

    /// Toggle an existing block off or apply the requested format to unformatted lines.
    /// A code suggestion retains its original fence and literal replacement text.
    pub(super) fn toggle_rich_block(
        &mut self,
        format: MarkdownFormat,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(rich) = &self.markdown.rich else {
            return false;
        };
        let selection = self.selected_range.clone();
        if matches!(format, MarkdownFormat::CodeBlock)
            && let Some(fence) = rich.blocks.enclosing_fence(&selection)
        {
            if !fence.suggestion {
                let cuts = vec![
                    (fence.outer.start..fence.code.start, String::new()),
                    (fence.code.end..fence.outer.end, String::new()),
                ];
                self.edit_block_markers(&cuts, cx);
            }
            return true;
        }
        let prefix = match format {
            MarkdownFormat::Heading => "### ",
            MarkdownFormat::Quote => "> ",
            MarkdownFormat::Bullets => "- ",
            MarkdownFormat::Checklist => "- [ ] ",
            MarkdownFormat::Numbered | MarkdownFormat::CodeBlock => "",
            _ => return false,
        };
        if format == MarkdownFormat::CodeBlock {
            return false; // Normal insertion already chooses a fence longer than the selected code.
        }
        let remove = self.rich_block_active(format);
        let edits = touched_rows(self.text(), &selection)
            .enumerate()
            .filter_map(|(index, row)| {
                if let Some(existing) = format
                    .block_format()
                    .and_then(|kind| rich.blocks.prefix(row, kind))
                {
                    return remove.then(|| (existing.clone(), String::new()));
                }
                let marker = if is_list(format) {
                    rich.blocks.list_prefix(row).cloned()
                } else {
                    None
                };
                let replacement = if format == MarkdownFormat::Numbered {
                    format!("{}. ", index + 1)
                } else {
                    prefix.to_string()
                };
                Some((marker.unwrap_or(row..row), replacement))
            })
            .collect::<Vec<_>>();
        self.edit_block_markers(&edits, cx);
        true
    }

    /// Applies all marker changes as one undo step, retaining the caret and selection direction.
    fn edit_block_markers(&mut self, edits: &[(Range<usize>, String)], cx: &mut Context<Self>) {
        let (Some((first, _)), Some((last, _))) = (edits.first(), edits.last()) else {
            return;
        };
        let text = self.text();
        let range = first.start..last.end;
        let mut cursor = range.start;
        let mut replacement = String::new();
        for (marker, insert) in edits {
            replacement.push_str(&text[cursor..marker.start]);
            replacement.push_str(insert);
            cursor = marker.end;
        }
        let selection =
            remap(self.selected_range.start, edits)..remap(self.selected_range.end, edits);
        let reversed = self.selection_reversed;
        self.replace(range, &replacement, cx);
        self.selected_range = selection;
        self.selection_reversed = reversed;
        self.finish_edit_selection();
    }
}

/// Translates an original source boundary through sorted, nonoverlapping replacements.
fn remap(offset: usize, edits: &[(Range<usize>, String)]) -> usize {
    let mut mapped = offset;
    for (marker, insert) in edits {
        if offset < marker.start {
            break;
        }
        mapped -= offset.min(marker.end) - marker.start;
        mapped += insert.len();
    }
    mapped
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    /// Each block button toggles the same selection back to its original source, including Unicode.
    #[gpui::test]
    fn rich_block_formats_toggle_and_preserve_selection(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::comment("", cx));
        editor.update(cx, |area, cx| {
            area.set_github_markdown(true, cx);
            for format in [
                MarkdownFormat::Heading,
                MarkdownFormat::Quote,
                MarkdownFormat::Bullets,
                MarkdownFormat::Numbered,
                MarkdownFormat::Checklist,
                MarkdownFormat::CodeBlock,
            ] {
                let source = "naïve α\nβeta";
                area.set_text(source, cx);
                area.selected_range = 0..source.len();
                format.apply(area, cx);
                assert!(
                    area.rich_format_active(format),
                    "{}: {:?}",
                    format.label(),
                    area.text()
                );
                format.apply(area, cx);
                assert_eq!(area.text(), source, "{}", format.label());
                assert_eq!(area.selected_range, 0..source.len());
                assert!(!area.rich_format_active(format));

                area.set_text("", cx);
                format.apply(area, cx);
                assert!(
                    area.rich_format_active(format),
                    "{} at a caret: {:?}",
                    format.label(),
                    area.text()
                );
                format.apply(area, cx);
                assert_eq!(area.text(), "");
                assert_eq!(area.selected_range, 0..0);
            }
        });
    }

    /// A mixed list normalizes every selected row without duplicating markers or damaging a checked item.
    #[gpui::test]
    fn changes_between_list_formats_preserve_other_lines(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::comment("", cx));
        editor.update(cx, |area, cx| {
            area.set_github_markdown(true, cx);
            area.set_text("- [X] checked\n* other\nnext", cx);
            area.selected_range = 6..area.text().find("next").unwrap();
            area.selection_reversed = true;
            MarkdownFormat::Checklist.apply(area, cx);
            assert_eq!(area.text(), "- [X] checked\n- [ ] other\nnext");
            assert!(area.selection_reversed);
            MarkdownFormat::Numbered.apply(area, cx);
            assert_eq!(area.text(), "1. checked\n2. other\nnext");
            MarkdownFormat::Numbered.apply(area, cx);
            assert_eq!(area.text(), "checked\nother\nnext");
            assert_eq!(
                &area.text()[area.selected_range.clone()],
                "checked\nother\n"
            );
        });
    }

    /// Existing fences retain literal source and the replacement range of a GitHub suggestion.
    #[gpui::test]
    fn block_controls_preserve_literal_code(cx: &mut TestAppContext) {
        let editor = cx.new(|cx| TextArea::comment("", cx));
        editor.update(cx, |area, cx| {
            area.set_github_markdown(true, cx);
            let raw = "before\n\n~~~rust\n- not a list\n~~~\n\nafter";
            area.set_text(raw, cx);
            let position = raw.find("list").unwrap();
            area.selected_range = position..position;
            assert!(!area.rich_format_active(MarkdownFormat::Bullets));
            MarkdownFormat::Bullets.apply(area, cx);
            assert_eq!(area.text(), raw);
            MarkdownFormat::CodeBlock.apply(area, cx);
            assert_eq!(area.text(), "before\n\n- not a list\n\nafter");
            let position = area.text().find("list").unwrap();
            assert_eq!(area.selected_range, position..position);

            area.set_text("", cx);
            area.insert_suggestion("λ", cx);
            let body = area.text().to_owned();
            MarkdownFormat::CodeBlock.apply(area, cx);
            assert_eq!(area.text(), body);
        });
    }
}
