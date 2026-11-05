use std::ops::Range;

use gpui::{FontStyle, FontWeight, HighlightStyle, Rgba, rgb};
use qc_syntax::SyntaxSpan;

/// The text highlights of one diff line: syntax colors from `syntax`, and
/// `emphasis` behind the `changed` ranges. Both are cut at each other's
/// edges, so the result is sorted and never overlaps, as gpui's text
/// highlights require. Ranges are clamped to the line's `length`.
pub fn code_highlights(
    length: usize,
    syntax: &[SyntaxSpan],
    changed: &[Range<usize>],
    emphasis: Rgba,
) -> Vec<(Range<usize>, HighlightStyle)> {
    // Every place where the color or the change mark may switch.
    let mut edges = vec![0, length];
    for span in syntax {
        edges.extend([span.range.start.min(length), span.range.end.min(length)]);
    }
    for range in changed {
        edges.extend([range.start.min(length), range.end.min(length)]);
    }
    edges.sort_unstable();
    edges.dedup();

    // Style each piece between two edges; plain, unchanged pieces need no highlight.
    edges
        .windows(2)
        .filter_map(|pair| {
            let piece = pair[0]..pair[1];
            let span = syntax
                .iter()
                .find(|span| span.range.start <= piece.start && piece.end <= span.range.end);
            let is_changed = changed
                .iter()
                .any(|range| range.start <= piece.start && piece.end <= range.end);
            if span.is_none() && !is_changed {
                return None;
            }
            let mut highlight = HighlightStyle::default();
            if let Some(span) = span {
                highlight.color = Some(rgb(span.color.to_rgb()).into());
                if span.is_bold {
                    highlight.font_weight = Some(FontWeight::BOLD);
                }
                if span.is_italic {
                    highlight.font_style = Some(FontStyle::Italic);
                }
            }
            if is_changed {
                highlight.background_color = Some(emphasis.into());
            }
            Some((piece, highlight))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use qc_syntax::SyntaxColor;

    use super::*;

    const KEYWORD: SyntaxColor = SyntaxColor {
        red: 0xc6,
        green: 0x78,
        blue: 0xdd,
    };

    fn span(range: Range<usize>) -> SyntaxSpan {
        SyntaxSpan {
            range,
            color: KEYWORD,
            is_bold: false,
            is_italic: false,
        }
    }

    /// A change that starts inside a colored token splits it; both halves keep the color.
    #[test]
    fn change_splits_syntax_span() {
        let emphasis = rgb(0x00_33_00);
        let changed: [Range<usize>; 1] = [Range { start: 4, end: 8 }];
        let highlights = code_highlights(10, &[span(0..6)], &changed, emphasis);
        let pieces: Vec<_> = highlights
            .iter()
            .map(|(range, style)| {
                (
                    range.clone(),
                    style.color.is_some(),
                    style.background_color.is_some(),
                )
            })
            .collect();
        assert_eq!(
            pieces,
            [(0..4, true, false), (4..6, true, true), (6..8, false, true)]
        );
    }

    /// Without colors or changes there is nothing to highlight.
    #[test]
    fn plain_line_has_no_highlights() {
        assert_eq!(code_highlights(5, &[], &[], rgb(0)).len(), 0);
    }
}
