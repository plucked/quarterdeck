use std::collections::BTreeMap;
use std::ops::Range;

use pulldown_cmark::{Event, Tag};

use crate::{BlockFormat, MarkdownFence};

/// Delimiters that the Markdown parser recognized as formatting for a logical line.
#[derive(Default)]
pub struct RichBlocks {
    prefixes: BTreeMap<usize, Vec<(BlockFormat, Range<usize>)>>,
    fences: Vec<MarkdownFence>,
    items: Vec<(Range<usize>, Range<usize>)>,
}

impl RichBlocks {
    /// Records only parsed block markers, keeping similar-looking text inside code literal.
    pub(crate) fn record(&mut self, event: &Event<'_>, outer: &Range<usize>, source: &str) {
        let raw = &source[outer.clone()];
        let marker = match event {
            Event::Start(Tag::Heading { .. }) if raw.starts_with('#') => {
                let count = raw.bytes().take_while(|byte| *byte == b'#').count();
                Some((BlockFormat::Heading, count + padding(&raw[count..])))
            }
            Event::Start(Tag::Item) => list_marker(raw),
            _ => None,
        };
        if let Some((format, len)) = marker {
            let marker = outer.start..outer.start + len;
            if is_list(format) {
                let end = outer.end - usize::from(raw.ends_with('\n'));
                self.items.push((outer.start..end, marker.clone()));
            }
            self.add(source, format, marker);
        }
    }

    fn add(&mut self, source: &str, format: BlockFormat, marker: Range<usize>) {
        let row = source[..marker.start]
            .rfind('\n')
            .map_or(0, |newline| newline + 1);
        self.prefixes.entry(row).or_default().push((format, marker));
    }

    /// Uses the same quote marker and optional following space that the rich surface displays.
    pub(crate) fn quote(&mut self, source: &str, marker: Range<usize>) {
        let end = marker.end + usize::from(source[marker.end..].starts_with([' ', '\t']));
        self.add(source, BlockFormat::Quote, marker.start..end);
    }

    pub(crate) fn record_fence(&mut self, fence: MarkdownFence) {
        self.fences.push(fence);
    }

    pub fn fences(&self) -> &[MarkdownFence] {
        &self.fences
    }

    /// The innermost list item containing a byte boundary, including the end of its last line.
    pub fn list_item_at(&self, offset: usize) -> Option<&Range<usize>> {
        self.items.iter().rev().find_map(|(span, marker)| {
            (span.start <= offset && offset <= span.end).then_some(marker)
        })
    }

    pub fn enclosing_fence(&self, selection: &Range<usize>) -> Option<&MarkdownFence> {
        self.fences
            .iter()
            .find(|fence| fence.outer.start <= selection.start && selection.end <= fence.outer.end)
    }

    /// The actual source delimiters for a toolbar style, including alternate list and heading syntax.
    pub fn prefix(&self, row: usize, format: BlockFormat) -> Option<&Range<usize>> {
        self.prefixes
            .get(&row)?
            .iter()
            .find_map(|(kind, marker)| (*kind == format).then_some(marker))
    }

    /// The first source byte after every block marker on this row, including nested quotes.
    pub fn content_start(&self, row: usize) -> usize {
        self.prefixes
            .get(&row)
            .and_then(|prefixes| prefixes.iter().map(|(_, marker)| marker.end).max())
            .unwrap_or(row)
    }

    pub fn list_prefix(&self, row: usize) -> Option<&Range<usize>> {
        self.prefixes
            .get(&row)?
            .iter()
            .find_map(|(kind, marker)| is_list(*kind).then_some(marker))
    }
}

/// Width of the whitespace separating a Markdown marker from its text.
fn padding(raw: &str) -> usize {
    raw.bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count()
}

fn list_marker(raw: &str) -> Option<(BlockFormat, usize)> {
    let count = raw.bytes().take_while(u8::is_ascii_digit).count();
    let (mut format, marker) = if count > 0 && raw[count..].starts_with(['.', ')']) {
        (BlockFormat::Numbered, count + 1)
    } else if raw.starts_with(['-', '+', '*']) {
        (BlockFormat::Bullets, 1)
    } else {
        return None;
    };
    let mut end = marker + padding(&raw[marker..]);
    if ["[ ]", "[x]", "[X]"]
        .iter()
        .any(|mark| raw[end..].starts_with(mark))
        && raw[end + 3..].starts_with(char::is_whitespace)
    {
        format = BlockFormat::Checklist;
        end += 3 + padding(&raw[end + 3..]);
    }
    Some((format, end))
}

fn is_list(format: BlockFormat) -> bool {
    matches!(
        format,
        BlockFormat::Bullets | BlockFormat::Numbered | BlockFormat::Checklist
    )
}
