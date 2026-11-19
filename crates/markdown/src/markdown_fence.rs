use std::borrow::Cow;
use std::collections::BTreeMap;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

/// Original source boundaries of a fenced code block. A suggestion targets diff rows
/// only when GitHub sees its fence at the document root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkdownFence {
    pub outer: Range<usize>,
    pub code: Range<usize>,
    pub opening: Range<usize>,
    pub closing: Option<Range<usize>>,
    pub suggestion: bool,
    /// Continuation of the opening line's quote and list containers, outside the literal code.
    container_prefix: String,
    /// Additional structural spaces needed before literal whitespace at each under-indented payload start.
    missing_padding: BTreeMap<usize, usize>,
}

impl MarkdownFence {
    /// Finds a suggestion in Markdown source when a rich projection is unavailable.
    pub fn suggestion_in(source: &str) -> Option<Self> {
        if !source.contains("suggestion") {
            return None;
        }
        let mut events = Parser::new(source).into_offset_iter();
        let mut depth = 0;
        while let Some((event, outer)) = events.next() {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info)))
                    if depth == 0 && info.as_ref() == "suggestion" =>
                {
                    return Some(Self::from_events(source, outer, &info, true, &mut events));
                }
                Event::Start(_) => depth += 1,
                Event::End(_) => depth -= 1,
                _ => {}
            }
        }
        None
    }

    /// The parser has already distinguished payload from a valid closing fence,
    /// including nested container prefixes, indentation, marker character and length.
    pub(crate) fn from_events<'a>(
        source: &str,
        outer: Range<usize>,
        info: &str,
        top_level: bool,
        events: &mut impl Iterator<Item = (Event<'a>, Range<usize>)>,
    ) -> Self {
        let raw = &source[outer.clone()];
        let marker = raw.as_bytes()[0];
        let opening_len = raw.bytes().take_while(|byte| *byte == marker).count();
        let opening = outer.start..outer.start + opening_len;
        let start = raw
            .find('\n')
            .map_or(outer.end, |newline| outer.start + newline + 1);
        // Continue the opening line's containers. List marks become padding on later lines;
        // a snug quote marker gets a separator so it never consumes literal code whitespace.
        let mut container_prefix = String::new();
        if !top_level {
            let preceding = &source[..opening.start];
            let line = preceding.rsplit('\n').next().unwrap_or_default();
            container_prefix = crate::container_continuation(line);
            if container_prefix.ends_with('>') {
                container_prefix.push(' ');
            }
        }
        let indentation = indentation_after_quote(&container_prefix);
        let mut missing_padding = BTreeMap::new();
        let mut first_payload = None;
        let mut payload_end = start;
        for (event, span) in events.by_ref() {
            match event {
                Event::Text(_) => {
                    // The parser gives the first literal byte after any optional quote space,
                    // list padding and fence indentation actually present on this line.
                    if indentation > 0 {
                        let prefix = source[..span.start].rsplit('\n').next().unwrap_or_default();
                        if prefix
                            .bytes()
                            .all(|byte| matches!(byte, b' ' | b'\t' | b'>'))
                        {
                            let missing =
                                indentation.saturating_sub(indentation_after_quote(prefix));
                            if missing > 0 {
                                missing_padding.insert(span.start, missing);
                            }
                        }
                    }
                    first_payload.get_or_insert(span.start);
                    payload_end = span.end;
                }
                Event::End(TagEnd::CodeBlock) => break,
                _ => {}
            }
        }
        let tail = source[payload_end..outer.end].trim_end_matches([' ', '\t', '\r', '\n']);
        let closing_len = tail
            .bytes()
            .rev()
            .take_while(|byte| *byte == marker)
            .count();
        let closing = (closing_len >= opening_len).then(|| {
            let end = payload_end + tail.len();
            end - closing_len..end
        });
        // A single line ending separates code from its closing fence. Earlier blank
        // lines belong to the proposed replacement; an unfinished block keeps them all.
        let payload = &source[start..payload_end];
        let end = if closing.is_some() {
            let code = payload
                .strip_suffix("\r\n")
                .or_else(|| payload.strip_suffix('\n'))
                .unwrap_or(payload);
            start + code.len()
        } else {
            outer.end
        };
        let start = if top_level {
            start
        } else {
            first_payload.unwrap_or(start).min(end)
        };
        Self {
            outer,
            code: start..end,
            opening,
            closing,
            suggestion: top_level && info == "suggestion",
            container_prefix,
            missing_padding,
        }
    }

    /// Encodes literal input for this fence, restoring the surrounding Markdown container
    /// at each new line, including the first code line of a compact or unfinished block.
    pub fn source_for_input<'a>(
        &self,
        source: &str,
        range: &Range<usize>,
        input: &'a str,
    ) -> Cow<'a, str> {
        if self.container_prefix.is_empty() {
            return Cow::Borrowed(input);
        }

        // Existing payload lines already carry their prefix before the insertion point.
        let header = &source[self.opening.end..self.code.start];
        let begins_line = !input.is_empty()
            && range.start == self.code.end
            && (source[..range.start].ends_with('\n')
                || (self.code.is_empty() && !header.contains('\n')));

        // Fill the missing structural padding before the parser sees literal whitespace.
        // Deleting a word can leave the same leading whitespace on its right.
        let following = if input.is_empty() {
            &source[range.end..]
        } else {
            input
        };
        let padding = if following.starts_with([' ', '\t']) {
            self.missing_padding.get(&range.start).copied().unwrap_or(0)
        } else {
            0
        };
        if !begins_line && padding == 0 && !input.contains('\n') {
            return Cow::Borrowed(input);
        }
        let mut encoded = String::new();
        if begins_line {
            encoded.push_str(&self.container_prefix);
        } else {
            encoded.extend(std::iter::repeat_n(' ', padding));
        }
        for line in input.split_inclusive('\n') {
            encoded.push_str(line);
            if line.ends_with('\n') {
                encoded.push_str(&self.container_prefix);
            }
        }
        Cow::Owned(encoded)
    }
}

/// Counts structural columns following the innermost quote, or from the start of a list line.
/// Quotes may move sideways on each line. `CommonMark` tab stops are four columns apart.
fn indentation_after_quote(prefix: &str) -> usize {
    const TAB_WIDTH: usize = 4;
    let mut column = 0;
    let mut quote_end = 0;
    for byte in prefix.bytes() {
        if byte == b'\t' {
            column += TAB_WIDTH - column % TAB_WIDTH;
        } else {
            column += 1;
            if byte == b'>' {
                quote_end = column;
            }
        }
    }
    column - quote_end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RichDocument;

    /// Rich editing and source-mode suggestion selection keep the same exact payload.
    #[test]
    fn projection_and_suggestion_selection_share_the_parser_payload() {
        for (source, expected, closed) in [
            ("````suggestion\na\n```", "a\n```", false),
            ("````suggestion\na\n```\n\n````", "a\n```\n", true),
            ("```suggestion\r\nα\r\n\r\n```", "α\r\n", true),
            ("```suggestion\n```", "", true),
            ("```suggestion\n\n```", "", true),
            ("```suggestion\na\n\n", "a\n\n", false),
            ("~~~suggestion\na\n    ~~~\n", "a\n    ~~~\n", false),
        ] {
            let rich = RichDocument::parse(source);
            let fence = MarkdownFence::suggestion_in(source).unwrap();
            assert_eq!(rich.blocks.fences(), std::slice::from_ref(&fence));
            assert_eq!(fence.closing.is_some(), closed, "{source:?}");
            assert_eq!(&source[fence.code], expected);
            assert_eq!(rich.text, expected);
        }
    }

    /// A fence nested in a quote or list is editable code and cannot anchor a suggestion.
    #[test]
    fn quoted_and_nested_code_examples_do_not_target_diff_rows() {
        for source in [
            "> ~~~suggestion\n> a\n> ~~~",
            "- item\n  ~~~suggestion\n  a\n  ~~~",
        ] {
            let rich = RichDocument::parse(source);
            let fence = &rich.blocks.fences()[0];
            assert!(!fence.suggestion);
            assert_eq!(&source[fence.closing.clone().unwrap()], "~~~");
            assert!(!rich.text.contains("~~~"));
            assert_eq!(&source[fence.code.clone()], "a");
            if source.starts_with('>') {
                assert_eq!(rich.text, "│ a");
            }
            assert!(MarkdownFence::suggestion_in(source).is_none());
        }
    }
}
