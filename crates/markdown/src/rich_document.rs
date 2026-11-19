use std::ops::Range;

use crate::inline_mark::inline_mark;
use crate::{InlineMark, InlineStyle, MarkdownFence, RichBlocks, StyledRun};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// An editable projection of Markdown. Unknown structures stay verbatim in the source and projection.
#[derive(Default)]
pub struct RichDocument {
    pub text: String,
    pub runs: Vec<StyledRun>,
    pub marks: Vec<InlineMark>,
    pub blocks: RichBlocks,
    /// Original offset of every displayed byte. Delimiters have no display width.
    source_bytes: Vec<usize>,
    source_end: usize,
    /// Empty fences still occupy an editable visual line with its own insertion boundary.
    empty_fences: Vec<(usize, usize)>,
    /// Replaced glyphs such as checkboxes may represent more than one source byte.
    glyph_ends: Vec<(usize, usize)>,
}

impl RichDocument {
    pub fn parse(source: &str) -> Self {
        let mut marks = Vec::new();
        let mut blocks = RichBlocks::default();
        let mut replacements: Vec<(Range<usize>, &'static str)> = Vec::new();
        let mut style_ranges = Vec::new();
        let mut quote_depth = 0;
        let mut depth = 0usize;
        let mut events = Parser::new_ext(
            source,
            Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
        )
        .into_offset_iter();
        while let Some((event, outer)) = events.next() {
            blocks.record(&event, &outer, source);
            let raw = &source[outer.clone()];
            let mark = match event {
                Event::Start(Tag::Heading { .. }) if raw.starts_with('#') => {
                    let prefix = raw.bytes().take_while(|byte| *byte == b'#').count();
                    let prefix = prefix
                        + raw[prefix..]
                            .bytes()
                            .take_while(|byte| matches!(byte, b' ' | b'\t'))
                            .count();
                    replacements.push((outer.start..outer.start + prefix, ""));
                    style_ranges.push((outer.clone(), InlineStyle::STRONG));
                    None
                }
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref language))) => {
                    let fence = MarkdownFence::from_events(
                        source,
                        outer.clone(),
                        language,
                        depth == 0,
                        &mut events,
                    );
                    replacements.push((outer.start..fence.code.start, ""));
                    replacements.push((fence.code.end..fence.outer.end, ""));
                    style_ranges.push((fence.code.clone(), InlineStyle::CODE));
                    blocks.record_fence(fence);
                    // from_events consumed both the opening and closing events.
                    continue;
                }
                Event::Start(Tag::BlockQuote(_)) => {
                    quote_depth += 1;
                    for marker in quote_markers(raw, outer.start, quote_depth) {
                        blocks.quote(source, marker.clone());
                        replacements.push((marker, "│"));
                    }
                    None
                }
                Event::End(TagEnd::BlockQuote(_)) => {
                    quote_depth -= 1;
                    None
                }
                Event::Start(Tag::Item)
                    if raw.starts_with(['-', '+', '*']) && raw.as_bytes().get(1) == Some(&b' ') =>
                {
                    replacements.push((outer.start..outer.start + 1, "•"));
                    None
                }
                Event::TaskListMarker(done) => {
                    replacements.push((outer.clone(), if done { "☑" } else { "☐" }));
                    None
                }
                _ => inline_mark(&event, &outer, raw),
            };
            match event {
                Event::Start(_) => depth += 1,
                Event::End(_) => depth -= 1,
                _ => {}
            }
            if let Some(mark) = mark {
                replacements.push((mark.outer.start..mark.inner.start, ""));
                replacements.push((mark.inner.end..mark.outer.end, ""));
                style_ranges.push((mark.inner.clone(), mark.style));
                marks.push(mark);
            }
        }

        let mut result = Self {
            marks,
            blocks,
            source_end: source.len(),
            ..Self::default()
        };
        result.project_source(source, replacements);
        result.build_styled_runs(style_ranges);
        result
    }

    /// Copy the visible source in order, replacing delimiters with their displayed glyphs.
    fn project_source(
        &mut self,
        source: &str,
        mut replacements: Vec<(Range<usize>, &'static str)>,
    ) {
        replacements.sort_by_key(|(range, _)| (range.start, range.end));
        let mut start = 0;
        for (clip, replacement) in replacements {
            if clip.start < start || clip.is_empty() {
                continue;
            }
            self.copy(source, start..clip.start);
            self.text.push_str(replacement);
            if !replacement.is_empty() {
                self.glyph_ends.push((self.text.len(), clip.end));
            }
            self.source_bytes
                .extend(std::iter::repeat_n(clip.start, replacement.len()));
            start = clip.end;
        }
        self.copy(source, start..source.len());
        self.empty_fences = self
            .blocks
            .fences()
            .iter()
            .filter(|fence| fence.code.is_empty())
            .map(|fence| (self.visible(fence.code.start), fence.code.start))
            .collect();
    }

    /// Sweeps style boundaries once so typing into a long, formatted draft stays responsive.
    fn build_styled_runs(&mut self, ranges: Vec<(Range<usize>, InlineStyle)>) {
        let mut edges = Vec::with_capacity(ranges.len() * 2);
        for (source, style) in ranges {
            let range = self.visible_range(&source);
            if !range.is_empty() {
                edges.push((range.start, style, true));
                edges.push((range.end, style, false));
            }
        }
        edges.sort_unstable_by_key(|(offset, _, _)| *offset);

        // A style can be nested inside itself, for example bold text inside a heading.
        let mut depths: Vec<(InlineStyle, usize)> = Vec::new();
        let mut previous = 0;
        for (offset, style, opening) in edges {
            if offset > previous {
                let active = depths
                    .iter()
                    .filter(|(_, depth)| *depth > 0)
                    .fold(InlineStyle::PLAIN, |active, (style, _)| active | *style);
                if !active.is_plain() {
                    self.runs.push(StyledRun {
                        range: previous..offset,
                        style: active,
                    });
                }
                previous = offset;
            }
            if let Some((_, depth)) = depths.iter_mut().find(|(active, _)| *active == style) {
                if opening {
                    *depth += 1;
                } else {
                    *depth -= 1;
                }
            } else {
                depths.push((style, 1));
            }
        }
    }

    fn copy(&mut self, source: &str, range: Range<usize>) {
        self.text.push_str(&source[range.clone()]);
        self.source_bytes.extend(range);
    }

    pub fn visible(&self, source: usize) -> usize {
        self.source_bytes.partition_point(|byte| *byte < source)
    }

    pub fn visible_range(&self, range: &Range<usize>) -> Range<usize> {
        self.visible(range.start)..self.visible(range.end)
    }

    /// The source boundary beside the visible character on the right, or after the preceding one.
    pub fn source(&self, display: usize, prefer_right: bool) -> usize {
        if let Ok(index) = self
            .empty_fences
            .binary_search_by_key(&display, |(offset, _)| *offset)
        {
            return self.empty_fences[index].1;
        }
        if prefer_right && let Some(byte) = self.source_bytes.get(display) {
            return *byte;
        }
        if let Ok(index) = self
            .glyph_ends
            .binary_search_by_key(&display, |(offset, _)| *offset)
        {
            return self.glyph_ends[index].1;
        }
        if display > 0
            && let Some(byte) = self.source_bytes.get(display - 1)
        {
            return *byte + 1;
        }
        self.source_bytes
            .first()
            .copied()
            .unwrap_or(self.source_end)
    }

    pub fn source_range(&self, visible: Range<usize>) -> Range<usize> {
        let start = self.source(visible.start, true);
        if visible.is_empty() {
            return start..start;
        }
        start..self.source(visible.end, false).max(start)
    }
}

/// Shows the quoted margin on each explicitly prefixed line. Extra `>` characters in code stay literal.
fn quote_markers(raw: &str, start: usize, depth: usize) -> impl Iterator<Item = Range<usize>> {
    let mut offset = start;
    raw.split_inclusive('\n')
        .enumerate()
        .filter_map(move |(index, line)| {
            let end = offset + line.len();
            offset = end;
            // The parser starts the first line at this quote's marker, including in nested quotes.
            let mut rest = line;
            for _ in 0..if index == 0 { 1 } else { depth } {
                rest = rest.trim_start_matches([' ', '\t']).strip_prefix('>')?;
            }
            let after_marker = end - rest.len();
            Some(after_marker - 1..after_marker)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A checkbox is one displayed character but its entire three-byte Markdown marker is editable.
    #[test]
    fn replaced_glyphs_cover_their_source_range() {
        let source = "- [x] café";
        let rich = RichDocument::parse(source);
        let start = rich.text.find('☑').unwrap();
        let range = rich.source_range(start..start + '☑'.len_utf8());
        assert_eq!(&source[range], "[x]");
    }

    /// An empty suggestion's first visual insertion is on the blank line between its fences.
    #[test]
    fn empty_fence_points_to_the_editable_line() {
        for source in [
            "```suggestion\n\n```",
            "Intro\n\n```suggestion\n```",
            "Intro\n\n```suggestion\n\n```\n\nAfter",
        ] {
            let rich = RichDocument::parse(source);
            let insertion = source.find("suggestion\n").unwrap() + "suggestion\n".len();
            let display = rich.visible(insertion);
            assert_eq!(rich.source(display, true), insertion);
            assert_eq!(rich.source(display, false), insertion);
        }
    }

    #[test]
    fn renders_quote_prefixes_without_changing_literal_greater_than_signs() {
        let source = "> > first\n> > café\n>\n> ```\n> > literal\n> ```\n\n2 > 1";
        let document = RichDocument::parse(source);
        assert!(document.text.starts_with("│ │ first\n│ │ café"));
        assert!(document.text.contains("> literal"));
        assert!(document.text.ends_with("2 > 1"));
        let marker = "│".len();
        assert_eq!(document.source_range(0..marker), 0..1);
    }
}
