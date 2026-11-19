use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use gpui::{ClipboardItem, Context};
use qd_markdown::InlineStyle;

use super::comment_history::EditSource;
use super::markdown_format::MarkdownFormat;
use super::{TextArea, TextAreaEvent};
use qd_markdown::RichDocument;

pub(super) const RICH_CLIPBOARD: &str = "quarterdeck/markdown\n";

impl MarkdownFormat {
    fn inline(self) -> Option<(InlineStyle, &'static str)> {
        Some(match self {
            Self::Bold => (InlineStyle::STRONG, "**"),
            Self::Italic => (InlineStyle::EMPHASIS, "*"),
            Self::Strike => (InlineStyle::STRIKETHROUGH, "~~"),
            Self::Code => (InlineStyle::CODE, "`"),
            _ => return None,
        })
    }
}

impl TextArea {
    /// Enables visual editing for a GitHub composer and restores plain source for local notes.
    pub fn set_github_markdown(&mut self, github: bool, cx: &mut Context<Self>) {
        self.markdown.github = github;
        self.markdown.link = None;
        self.markdown.preview = None;
        self.markdown.pending.clear();
        self.refresh_rich();
        self.normalize_rich_selection();
        cx.emit(TextAreaEvent::LayoutChanged);
        cx.notify();
    }

    pub(super) fn refresh_rich(&mut self) {
        self.markdown.rich = (self.markdown.github && !self.markdown.source_mode)
            .then(|| Arc::new(RichDocument::parse(self.text())));
        self.markdown.suggestion = match &self.markdown.rich {
            Some(rich) => rich
                .blocks
                .fences()
                .iter()
                .find(|fence| fence.suggestion)
                .cloned(),
            None if self.is_comment() => qd_markdown::MarkdownFence::suggestion_in(self.text()),
            None => None,
        };
        self.markdown.suggestion_in_history |= self.markdown.suggestion.is_some();
    }

    /// The text the cursor, mouse and input method operate on in this writing mode.
    pub(super) fn editing_text(&self) -> &str {
        self.markdown
            .rich
            .as_ref()
            .map_or_else(|| self.text(), |rich| rich.text.as_str())
    }

    pub(super) fn to_visible(&self, source: usize) -> usize {
        self.markdown
            .rich
            .as_ref()
            .map_or(source, |rich| rich.visible(source))
    }

    pub(super) fn to_source(&self, display: usize, prefer_right: bool) -> usize {
        self.markdown
            .rich
            .as_ref()
            .map_or(display, |rich| rich.source(display, prefer_right))
    }

    pub(super) fn visible_range(&self, source: &Range<usize>) -> Range<usize> {
        self.to_visible(source.start)..self.to_visible(source.end)
    }

    pub(super) fn source_range(&self, display: Range<usize>) -> Range<usize> {
        self.markdown.rich.as_ref().map_or_else(
            || display.clone(),
            |rich| rich.source_range(display.clone()),
        )
    }

    /// Source mode can place the caret inside syntax that Rich text hides. Resolve both
    /// ends against the displayed characters before the next edit or IME composition.
    pub(super) fn normalize_rich_selection(&mut self) {
        let Some(rich) = &self.markdown.rich else {
            return;
        };
        let selected = rich.visible_range(&self.selected_range);
        if self.selected_range == (0..self.text().len()) && !selected.is_empty() {
            return;
        }
        let boundary = |offset, display, prefer_right| {
            let left = rich.source(display, false);
            let right = rich.source(display, true);
            if offset == left
                || offset == right
                || rich
                    .marks
                    .iter()
                    .any(|mark| offset == mark.outer.start || offset == mark.outer.end)
            {
                offset
            } else if prefer_right {
                right
            } else {
                left
            }
        };
        if selected.is_empty() {
            let caret = boundary(self.cursor_offset(), selected.start, true);
            self.selected_range = caret..caret;
            self.selection_reversed = false;
        } else {
            self.selected_range = boundary(self.selected_range.start, selected.start, true)
                ..boundary(self.selected_range.end, selected.end, false);
        }
    }

    /// Cursor movement follows visible characters rather than stepping through invisible delimiters.
    pub(super) fn cursor_boundary(&self, boundary: fn(&str, usize) -> usize) -> usize {
        let caret = self.to_visible(self.cursor_offset());
        let next = boundary(self.editing_text(), caret);
        if next == caret {
            return self.cursor_offset();
        }
        self.to_source(next, next < caret)
    }

    pub(super) fn sync_rich_selection(&mut self, cx: &mut Context<Self>) {
        self.markdown.pending.clear();
        if self.markdown.rich.is_some() {
            cx.emit(TextAreaEvent::FormattingChanged);
            cx.notify();
        }
    }

    pub(super) fn rich_format_active(&self, format: MarkdownFormat) -> bool {
        let Some(rich) = &self.markdown.rich else {
            return false;
        };
        let style = match format {
            MarkdownFormat::Link => InlineStyle::LINK,
            _ => match format.inline() {
                Some((style, _)) => style,
                None => return self.rich_block_active(format),
            },
        };
        if let Some((_, active)) = self
            .markdown
            .pending
            .iter()
            .find(|(pending, _)| *pending == style)
        {
            return *active;
        }
        let range = self.visible_range(&self.selected_range);
        if range.is_empty() {
            let caret = self.cursor_offset();
            rich.marks.iter().any(|mark| {
                mark.style == style && mark.inner.start <= caret && caret <= mark.inner.end
            })
        } else {
            super::rich_inline_selection::selection_has_style(self.text(), rich, range, style)
        }
    }

    /// Toggles an inline style on the typing caret or removes it from a formatted selection.
    pub(super) fn handle_rich_format(
        &mut self,
        format: MarkdownFormat,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(rich) = &self.markdown.rich else {
            return false;
        };
        let Some((style, _)) = format.inline() else {
            return false;
        };
        let active = self.rich_format_active(format);
        if self.selected_range.is_empty() {
            self.markdown
                .pending
                .retain(|(pending, _)| *pending != style);
            self.markdown.pending.push((style, !active));
            cx.emit(TextAreaEvent::FormattingChanged);
            cx.notify();
            return true;
        }
        let selected = rich.visible_range(&self.selected_range);
        let Some((range, replacement)) = super::rich_inline_selection::toggle_inline_style(
            self.text(),
            rich,
            selected.clone(),
            style,
            active,
        ) else {
            return true;
        };
        let reversed = self.selection_reversed;
        self.replace(range, &replacement, cx);
        self.selected_range = self.source_range(selected);
        self.selection_reversed = reversed;
        self.finish_edit_selection();
        true
    }

    /// Wrapping a complete styled selection includes its existing delimiters but records the original
    /// caret and selection in undo history.
    pub(super) fn wrapping_selection(&self) -> Range<usize> {
        // A full styled selection includes its inner delimiters when wrapped in another style.
        let Some(rich) = self.markdown.rich.as_ref() else {
            return self.selected_range.clone();
        };
        let selected = rich.visible_range(&self.selected_range);
        let mut source = rich.source_range(selected.clone());
        for mark in &rich.marks {
            let inner = rich.visible_range(&mark.inner);
            if selected.start <= inner.start && inner.end <= selected.end {
                source.start = source.start.min(mark.outer.start);
                source.end = source.end.max(mark.outer.end);
            }
        }
        source
    }

    /// Translates a visible edit back to source while preserving markup around the unaffected text.
    pub(super) fn edit_input(
        &mut self,
        mut range: Range<usize>,
        text: &str,
        source: EditSource,
        cx: &mut Context<Self>,
    ) -> Range<usize> {
        let Some(rich) = self.markdown.rich.clone() else {
            return self.replace_from(range, text, source, cx);
        };
        if let Some(inserted) = self.edit_fenced_code(&range, text, source, cx) {
            return inserted;
        }
        let mut before = String::new();
        let mut after = String::new();
        preserve_crossing_styles(&rich, self.text(), &mut range, text, &mut after);
        let mut input = Cow::Borrowed(text);
        preserve_crossing_fences(
            &rich,
            self.text(),
            &mut range,
            &mut input,
            &mut before,
            &mut after,
        );
        let text = input.as_ref();
        self.typing_delimiters(&rich, &mut range, text, &mut before, &mut after);
        if range.is_empty()
            && matches!(source, EditSource::Command | EditSource::Typing)
            && let Some((range, replacement, caret)) =
                self.extend_typing_run(&rich, range.start, text, &before, &after)
        {
            let inserted = self.replace_from(range, &replacement, source, cx);
            self.selected_range = caret..caret;
            self.finish_edit_selection();
            return inserted;
        }
        // Markdown inline marks cannot span a paragraph boundary. Wrap each pasted paragraph
        // separately while retaining the writer's line breaks and current typing style.
        if range.is_empty()
            && matches!(source, EditSource::Typing | EditSource::Command)
            && !before.is_empty()
            && !after.is_empty()
            && text.contains('\n')
            && self.markdown.pending.iter().all(|(_, enabled)| *enabled)
        {
            let marked = text
                .split('\n')
                .map(|line| marked_fragment(line, &before, &after))
                .collect::<Vec<_>>()
                .join("\n");
            let inserted = self.replace_from(range, &marked, source, cx);
            let caret = inserted.end
                - if marked.ends_with(&after) {
                    after.len()
                } else {
                    0
                };
            self.selected_range = caret..caret;
            self.finish_edit_selection();
            return inserted;
        }
        // GitHub requires emphasis delimiters to touch the styled words. Leave boundary spaces
        // outside the markers in source, while remembering the formatting for the next keystroke.
        if range.is_empty() && matches!(source, EditSource::Typing | EditSource::Command) {
            let trimmed = text.trim_end();
            let trailing = &text[trimmed.len()..];
            let leading = if before.is_empty() {
                0
            } else {
                trimmed.len() - trimmed.trim_start().len()
            };
            if !trailing.is_empty() {
                let mut end = range.end;
                while let Some(mark) = rich.marks.iter().find(|mark| {
                    mark.inner.end == end && mark.outer.end > end && mark.style != InlineStyle::LINK
                }) {
                    after.push_str(&self.text()[end..mark.outer.end]);
                    if !self
                        .markdown
                        .pending
                        .iter()
                        .any(|(style, _)| *style == mark.style)
                    {
                        self.markdown.pending.push((mark.style, true));
                    }
                    end = mark.outer.end;
                }
                range.end = end;
            }
            if leading > 0 || !trailing.is_empty() {
                let words = &trimmed[leading..];
                let replacement = format!("{}{before}{words}{after}{trailing}", &text[..leading]);
                let caret = range.start + replacement.len()
                    - if trailing.is_empty() { after.len() } else { 0 };
                let inserted = self.replace_from(range, &replacement, source, cx);
                self.selected_range = caret..caret;
                self.finish_edit_selection();
                return inserted;
            }
        }
        let start = range.start + before.len();
        let inserted = start..start + text.len();
        self.replace_from(range, &format!("{before}{text}{after}"), source, cx);
        self.selected_range = inserted.end..inserted.end;
        self.finish_edit_selection();
        inserted
    }

    /// A typed space waits outside the closing delimiter until the writer adds the next word.
    /// Reunite the words so their intervening space is styled and the phrase toggles as one run.
    fn extend_typing_run(
        &self,
        rich: &RichDocument,
        caret: usize,
        text: &str,
        opening: &str,
        closing: &str,
    ) -> Option<(Range<usize>, String, usize)> {
        if opening.is_empty()
            || text.trim().is_empty()
            || text.contains('\n')
            || !self.markdown.pending.iter().all(|(_, enabled)| *enabled)
        {
            return None;
        }
        let source = self.text();
        let previous = source[..caret].trim_end_matches([' ', '\t']).len();
        if previous == caret {
            return None;
        }
        let mark = rich.marks.iter().find(|mark| {
            mark.outer.end == previous
                && &source[mark.outer.start..mark.inner.start] == opening
                && &source[mark.inner.end..mark.outer.end] == closing
        })?;
        let words = text.trim_end();
        let spaces = &source[previous..caret];
        let trailing = &text[words.len()..];
        let replacement = format!("{spaces}{words}{closing}{trailing}");
        let caret = mark.inner.end + replacement.len()
            - if trailing.is_empty() {
                closing.len()
            } else {
                0
            };
        Some((mark.inner.end..previous + spaces.len(), replacement, caret))
    }

    fn typing_delimiters(
        &self,
        rich: &RichDocument,
        range: &mut Range<usize>,
        text: &str,
        before: &mut String,
        after: &mut String,
    ) {
        if !text.is_empty() && range.start == range.end {
            // Move outside a style at its boundary or split the run where typing changes style.
            for (style, enabled) in &self.markdown.pending {
                let mark = rich.marks.iter().find(|mark| {
                    mark.style == *style
                        && mark.inner.start <= range.start
                        && range.start <= mark.inner.end
                });
                if !enabled && let Some(mark) = mark {
                    if mark.inner.start == range.start {
                        *range = mark.outer.start..mark.outer.start;
                    } else if mark.inner.end == range.end {
                        *range = mark.outer.end..mark.outer.end;
                    } else {
                        let left = &self.text()[mark.inner.start..range.start];
                        let right = &self.text()[range.end..mark.inner.end];
                        let left_space = &left[left.trim_end().len()..];
                        let right_space = &right[..right.len() - right.trim_start().len()];
                        range.start -= left_space.len();
                        range.end += right_space.len();
                        before.push_str(&self.text()[mark.inner.end..mark.outer.end]);
                        before.push_str(left_space);
                        after.insert_str(0, &self.text()[mark.outer.start..mark.inner.start]);
                        after.insert_str(0, right_space);
                    }
                } else if *enabled && mark.is_none() && !text.trim().is_empty() {
                    let delimiter = style_delimiter(*style);
                    before.push_str(delimiter);
                    after.insert_str(0, delimiter);
                }
            }
        }
    }

    /// The public clipboard carries readable text. A second app-owned representation retains its styles.
    pub(super) fn selection_clipboard(&self) -> ClipboardItem {
        let Some(rich) = &self.markdown.rich else {
            return ClipboardItem::new_string(self.text()[self.selected_range.clone()].to_owned());
        };
        let visible = rich.visible_range(&self.selected_range);
        if self.selected_range == (0..self.text().len()) {
            return ClipboardItem::new_string_with_metadata(
                rich.text.clone(),
                format!("{RICH_CLIPBOARD}{}", self.text()),
            );
        }
        let source = rich.source_range(visible.clone());
        let mut content = self.text()[source.clone()].to_string();
        for mark in rich.marks.iter().rev() {
            if mark.inner.start <= source.start && source.start < mark.inner.end {
                content.insert_str(0, &self.text()[mark.outer.start..mark.inner.start]);
            }
        }
        for mark in rich.marks.iter().rev() {
            if mark.inner.start < source.end && source.end <= mark.inner.end {
                content.push_str(&self.text()[mark.inner.end..mark.outer.end]);
            }
        }
        ClipboardItem::new_string_with_metadata(
            rich.text[visible].to_string(),
            format!("{RICH_CLIPBOARD}{content}"),
        )
    }
}

/// Keep delimiters around untouched content on the two sides of a replacement.
fn preserve_crossing_styles(
    rich: &RichDocument,
    source: &str,
    range: &mut Range<usize>,
    text: &str,
    after: &mut String,
) {
    let selected = rich.visible_range(range);
    if range.start != range.end && !selected.is_empty() {
        if *range != (0..source.len()) {
            *range = rich.source_range(selected.clone());
        }
        for mark in &rich.marks {
            let content = rich.visible_range(&mark.inner);
            if selected.start <= content.start
                && content.end <= selected.end
                && (text.is_empty() || selected != content)
            {
                range.start = range.start.min(mark.outer.start);
                range.end = range.end.max(mark.outer.end);
            }
        }
        // Crossing one boundary retains the other delimiter around the unedited portion.
        for mark in rich.marks.iter().rev() {
            if mark.inner.start < range.start
                && range.start < mark.inner.end
                && mark.outer.end <= range.end
            {
                after.push_str(&source[mark.inner.end..mark.outer.end]);
            }
        }
        for mark in &rich.marks {
            if range.start <= mark.outer.start
                && mark.inner.start < range.end
                && range.end < mark.inner.end
            {
                after.push_str(&source[mark.outer.start..mark.inner.start]);
            }
        }
    }
}

/// Keep a partially selected code block on its own lines. A selection passing over all of
/// its code and into the surrounding prose removes the complete block, including its fences.
fn preserve_crossing_fences(
    rich: &RichDocument,
    source: &str,
    range: &mut Range<usize>,
    text: &mut Cow<'_, str>,
    before: &mut String,
    after: &mut String,
) {
    if range.start == range.end {
        return;
    }
    for fence in rich.blocks.fences() {
        if range.end < fence.outer.start
            || fence.outer.end < range.start
            || (fence.code.start <= range.start && range.end <= fence.code.end)
        {
            continue;
        }
        if range.end == fence.outer.start {
            let previous = after
                .chars()
                .next_back()
                .or_else(|| text.chars().next_back())
                .or_else(|| source[..range.start].chars().next_back());
            if previous.is_some_and(|ch| ch != '\n') {
                after.push('\n');
            }
        } else if range.start == fence.outer.end {
            let next = text
                .chars()
                .next()
                .or_else(|| after.chars().next())
                .or_else(|| source[range.end..].chars().next());
            if next.is_some_and(|ch| ch != '\n') {
                before.push('\n');
            }
        } else if range.start <= fence.code.start && fence.code.end <= range.end {
            range.start = range.start.min(fence.outer.start);
            range.end = range.end.max(fence.outer.end);
        } else if range.start < fence.code.start {
            range.start = range.start.min(fence.outer.start);
            let previous = after
                .chars()
                .next_back()
                .or_else(|| text.chars().next_back())
                .or_else(|| source[..range.start].chars().next_back());
            if previous.is_some_and(|ch| ch != '\n') {
                after.push('\n');
            }
            // Removing text before a run of backticks can also expose a closing fence.
            let suffix = &source[range.end..fence.code.end];
            let delimiters = qd_markdown::FenceDelimiters::for_code(fence, source, suffix);
            let opening_row = super::text_boundaries::line_start(source, fence.outer.start);
            after.push_str(&source[opening_row..fence.outer.start]);
            after.push_str(&delimiters.opening);
            after.push_str(suffix);
            after.push_str(&delimiters.closing);
            range.end = fence.outer.end;
        } else {
            range.end = range.end.max(fence.outer.end);
            if !after.is_empty()
                || (!source[range.end..].is_empty() && !source[range.end..].starts_with('\n'))
            {
                after.insert(0, '\n');
            }
            // Pasting a delimiter or deleting its trailing text can turn a literal
            // payload line into a closing fence. Size both ends from the surviving code.
            if let Cow::Owned(encoded) =
                fence.source_for_input(source, &(range.start..fence.code.end), text)
            {
                *text = Cow::Owned(encoded);
            }
            let prefix = &source[fence.code.start..range.start];
            let code = format!("{prefix}{text}");
            let delimiters = qd_markdown::FenceDelimiters::for_code(fence, source, &code);
            before.push_str(&delimiters.opening);
            before.push_str(prefix);
            range.start = fence.outer.start;
            after.insert_str(0, &delimiters.closing);
        }
    }
}

/// Markdown emphasis cannot touch an outer space. Moving those spaces outside keeps both its
/// display and the neighboring words intact when an existing formatting run is split.
fn marked_fragment(text: &str, opening: &str, closing: &str) -> String {
    let words = text.trim();
    if words.is_empty() {
        return text.to_string();
    }
    let start = text.len() - text.trim_start().len();
    format!(
        "{}{opening}{words}{closing}{}",
        &text[..start],
        &text[start + words.len()..]
    )
}

fn style_delimiter(style: InlineStyle) -> &'static str {
    if style == InlineStyle::STRONG {
        "**"
    } else if style == InlineStyle::EMPHASIS {
        "*"
    } else if style == InlineStyle::STRIKETHROUGH {
        "~~"
    } else {
        "`"
    }
}
