use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Tag};

use crate::InlineStyle;

/// A source delimiter and the characters it styles. Source ranges always refer to the original draft.
#[derive(Debug, Clone)]
pub struct InlineMark {
    pub outer: Range<usize>,
    pub inner: Range<usize>,
    pub style: InlineStyle,
    pub link_destination: Option<String>,
}

/// The original delimiters of an editable inline style. Unrecognized forms stay visible verbatim.
pub(crate) fn inline_mark(
    event: &Event<'_>,
    outer: &Range<usize>,
    raw: &str,
) -> Option<InlineMark> {
    let enclosed = |length, style| InlineMark {
        inner: outer.start + length..outer.end - length,
        outer: outer.clone(),
        style,
        link_destination: None,
    };
    match event {
        Event::Start(Tag::Strong) => Some(enclosed(2, InlineStyle::STRONG)),
        Event::Start(Tag::Emphasis) => Some(enclosed(1, InlineStyle::EMPHASIS)),
        Event::Start(Tag::Strikethrough) => Some(enclosed(
            raw.bytes().take_while(|byte| *byte == b'~').count().min(2),
            InlineStyle::STRIKETHROUGH,
        )),
        Event::Code(_) => {
            let ticks = raw.bytes().take_while(|byte| *byte == b'`').count();
            let mut mark = enclosed(ticks, InlineStyle::CODE);
            let content = &raw[ticks..raw.len() - ticks];
            if content.starts_with(' ') && content.ends_with(' ') && !content.trim().is_empty() {
                mark.inner.start += 1;
                mark.inner.end -= 1;
            }
            Some(mark)
        }
        Event::Start(Tag::Link {
            link_type: LinkType::Inline,
            dest_url,
            ..
        }) if raw.starts_with('[') => {
            // Identify the closing bracket of the label, including nested/escaped brackets.
            link_label_end(raw).map(|end| InlineMark {
                inner: outer.start + 1..outer.start + end,
                outer: outer.clone(),
                style: InlineStyle::LINK,
                link_destination: Some(dest_url.to_string()),
            })
        }
        _ => None,
    }
}

/// Returns the label boundary without consuming a bracket escaped inside a Markdown link.
fn link_label_end(raw: &str) -> Option<usize> {
    let mut depth = 0;
    let mut escaped = false;
    for (byte, character) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(byte);
                }
            }
            _ => {}
        }
    }
    None
}
