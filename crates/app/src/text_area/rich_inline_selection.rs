//! Rebuild the selected inline style while retaining surrounding words and other markup.

use std::ops::Range;

use qd_markdown::{InlineStyle, RichDocument};

use super::markdown_format::code_delimiter;
use super::text_boundaries::line_start;

/// Return a single source replacement so applying a style and merging its neighbors undo together.
pub(super) fn toggle_inline_style(
    source: &str,
    rich: &RichDocument,
    selected: Range<usize>,
    style: InlineStyle,
    remove: bool,
) -> Option<(Range<usize>, String)> {
    let markers: &[&str] = if style == InlineStyle::STRONG {
        &["**", "__"]
    } else if style == InlineStyle::EMPHASIS {
        &["*", "_"]
    } else if style == InlineStyle::STRIKETHROUGH {
        &["~~"]
    } else {
        &["`"]
    };
    // Adjacent Markdown delimiters can be reinterpreted by the parser (e.g. nested emphasis).
    // Try the alternative delimiter before accepting an edit that would expose literal markers.
    for marker in markers {
        let (range, replacement) =
            style_edit(source, rich, selected.clone(), style, remove, marker)?;
        let mut candidate = source.to_string();
        candidate.replace_range(range.clone(), &replacement);
        if RichDocument::parse(&candidate).text == rich.text {
            return Some((range, replacement));
        }
    }
    None
}

fn style_edit(
    source: &str,
    rich: &RichDocument,
    selected: Range<usize>,
    style: InlineStyle,
    remove: bool,
    marker: &str,
) -> Option<(Range<usize>, String)> {
    let mut affected = selected.clone();
    let mut edits = Vec::new();
    for mark in rich.marks.iter().filter(|mark| mark.style == style) {
        let content = rich.visible_range(&mark.inner);
        if content.end <= selected.start || selected.end <= content.start {
            continue;
        }
        affected.start = affected.start.min(content.start);
        affected.end = affected.end.max(content.end);
        edits.push((mark.outer.start..mark.inner.start, String::new()));
        edits.push((mark.inner.end..mark.outer.end, String::new()));
    }
    let mut wrap = |range| {
        for line in content_lines(source, rich, range) {
            wrap_line(source, rich, line, style, marker, &mut edits);
        }
    };
    if remove {
        wrap(affected.start..selected.start);
        wrap(selected.end..affected.end);
    } else {
        wrap(affected);
    }

    edits.sort_by_key(|(range, _)| (range.start, range.end));
    let start = edits.first()?.0.start;
    let end = edits.last()?.0.end;
    let mut replacement = String::new();
    let mut cursor = start;
    for (range, marker) in edits {
        replacement.push_str(&source[cursor..range.start]);
        replacement.push_str(&marker);
        cursor = range.end;
    }
    Some((start..end, replacement))
}

/// Selectable inline content on each line, excluding parsed block prefixes and boundary spaces.
fn content_lines<'a>(
    source: &'a str,
    rich: &'a RichDocument,
    visible: Range<usize>,
) -> impl Iterator<Item = Range<usize>> + 'a {
    rich.text[visible.clone()]
        .split_inclusive('\n')
        .scan(visible.start, move |start, line| {
            let end = *start + line.len();
            let row = line_start(source, rich.source(*start, true));
            let content = rich
                .visible(rich.blocks.content_start(row))
                .clamp(*start, end);
            *start = end;
            let span = &rich.text[content..end];
            let first = content + span.len() - span.trim_start().len();
            Some(first..first + span.trim().len())
        })
        .filter(|line| !line.is_empty())
}

/// Coverage uses the same content as the edit, so the button can toggle back off across rows.
pub(super) fn selection_has_style(
    source: &str,
    rich: &RichDocument,
    visible: Range<usize>,
    style: InlineStyle,
) -> bool {
    let mut lines = content_lines(source, rich, visible).peekable();
    lines.peek().is_some()
        && lines.all(|line| {
            let mut end = line.start;
            let first = rich.runs.partition_point(|run| run.range.end <= end);
            for run in &rich.runs[first..] {
                if run.range.start > end || !run.style.contains(style) {
                    return false;
                }
                end = run.range.end;
                if end >= line.end {
                    return true;
                }
            }
            false
        })
}

/// Split at crossing inline styles so Markdown delimiters nest correctly.
/// Each byte of the existing content stays in source, including link destinations.
fn wrap_line(
    raw: &str,
    rich: &RichDocument,
    visible: Range<usize>,
    style: InlineStyle,
    marker: &str,
    edits: &mut Vec<(Range<usize>, String)>,
) {
    let mut boundaries = vec![visible.start, visible.end];
    for mark in rich
        .marks
        .iter()
        .filter(|mark| mark.style != style && mark.style != InlineStyle::CODE)
    {
        let content = rich.visible_range(&mark.inner);
        // Code makes nested Markdown literal. Put its delimiters inside every existing style.
        let split = style == InlineStyle::CODE;
        if (split || visible.end < content.end)
            && visible.start < content.start
            && content.start < visible.end
        {
            boundaries.push(content.start);
        }
        if (split || content.start < visible.start)
            && visible.start < content.end
            && content.end < visible.end
        {
            boundaries.push(content.end);
        }
    }
    boundaries.sort_unstable();
    boundaries.dedup();
    for ends in boundaries.windows(2) {
        let span = &rich.text[ends[0]..ends[1]];
        let words = span.trim();
        if words.is_empty() {
            continue;
        }
        let start = ends[0] + span.len() - span.trim_start().len();
        let selected = start..start + words.len();
        let mut source = rich.source_range(selected.clone());
        let mut split_start = ("", "");
        let mut split_end = ("", "");
        for mark in &rich.marks {
            let content = rich.visible_range(&mark.inner);
            if mark.style == InlineStyle::CODE && style != InlineStyle::CODE {
                // Code must be the innermost mark. End and reopen the code span if a
                // new style starts or finishes between two of its literal characters.
                let split = (
                    &raw[mark.inner.end..mark.outer.end],
                    &raw[mark.outer.start..mark.inner.start],
                );
                if content.start == selected.start && selected.start < content.end {
                    source.start = source.start.min(mark.outer.start);
                } else if content.start < selected.start && selected.start < content.end {
                    split_start = split;
                }
                if selected.end == content.end && content.start < selected.end {
                    source.end = source.end.max(mark.outer.end);
                } else if content.start < selected.end && selected.end < content.end {
                    split_end = split;
                }
            }
            if style != InlineStyle::CODE
                && mark.style != style
                && selected.start <= content.start
                && content.end <= selected.end
            {
                source.start = source.start.min(mark.outer.start);
                source.end = source.end.max(mark.outer.end);
            }
        }
        let marker = if style == InlineStyle::CODE {
            code_delimiter(words, 1)
        } else {
            marker.to_owned()
        };
        let padding =
            if style == InlineStyle::CODE && (words.starts_with('`') || words.ends_with('`')) {
                " "
            } else {
                ""
            };
        edits.push((
            source.start..source.start,
            format!("{}{marker}{padding}{}", split_start.0, split_start.1),
        ));
        edits.push((
            source.end..source.end,
            format!("{}{padding}{marker}{}", split_end.0, split_end.1),
        ));
    }
}
