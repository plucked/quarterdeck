use std::ops::Range;

use crate::{InlineStyle, LinkRange, StyledRun};

/// A paragraph's (or heading's, or table cell's) text, flattened: one string
/// plus the styled byte ranges and link ranges over it.
///
/// Runs are sorted, never overlap and cover only non-plain text, so a
/// renderer can hand them to a text layout directly. Line breaks are `\n`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineText {
    text: String,
    runs: Vec<StyledRun>,
    links: Vec<LinkRange>,
}

impl InlineText {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn runs(&self) -> &[StyledRun] {
        &self.runs
    }

    pub fn links(&self) -> &[LinkRange] {
        &self.links
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Byte length of the text so far; where the next appended text starts.
    pub(crate) fn len(&self) -> usize {
        self.text.len()
    }

    /// Appends `text` in `style`, extending the previous run when it has the
    /// same style and ends where this text starts.
    pub(crate) fn push(&mut self, text: &str, style: InlineStyle) {
        let start = self.text.len();
        self.text.push_str(text);
        let end = self.text.len();
        if style.is_plain() || start == end {
            return;
        }
        match self.runs.last_mut() {
            Some(last) if last.style == style && last.range.end == start => last.range.end = end,
            Some(_) | None => self.runs.push(StyledRun {
                range: start..end,
                style,
            }),
        }
    }

    /// Records that `range` links to `url`; empty ranges are dropped.
    pub(crate) fn push_link(&mut self, range: Range<usize>, url: String) {
        if !range.is_empty() {
            self.links.push(LinkRange { range, url });
        }
    }

    /// Removes trailing whitespace (left by a trailing soft break), keeping
    /// runs and links inside the shortened text.
    pub(crate) fn trim_end(&mut self) {
        let trimmed_length = self.text.trim_end().len();
        self.text.truncate(trimmed_length);
        self.runs.retain_mut(|run| {
            run.range.end = run.range.end.min(trimmed_length);
            !run.range.is_empty()
        });
        self.links.retain_mut(|link| {
            link.range.end = link.range.end.min(trimmed_length);
            !link.range.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adjacent text of the same style becomes one run; plain text adds none.
    #[test]
    fn merges_adjacent_runs_of_same_style() {
        let mut inline = InlineText::default();
        inline.push("a ", InlineStyle::PLAIN);
        inline.push("bold", InlineStyle::STRONG);
        inline.push(" too", InlineStyle::STRONG);
        assert_eq!(inline.text(), "a bold too");
        assert_eq!(
            inline.runs(),
            [StyledRun {
                range: 2..10,
                style: InlineStyle::STRONG
            }]
        );
    }

    /// Trimming trailing whitespace shortens runs that reached into it.
    #[test]
    fn trim_end_clamps_runs() {
        let mut inline = InlineText::default();
        inline.push("word ", InlineStyle::EMPHASIS);
        inline.trim_end();
        assert_eq!(inline.text(), "word");
        assert_eq!(inline.runs()[0].range, 0..4);
    }
}
