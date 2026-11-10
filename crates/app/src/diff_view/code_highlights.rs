use std::ops::Range;

use gpui::{HighlightStyle, Rgba};
use qd_syntax::SyntaxSpan;

use crate::markdown_view::{covers, edge_pieces};
use crate::theme;

/// The text highlights of one diff line: syntax colors from `syntax`,
/// `emphasis` behind the `changed` ranges, and the selection background
/// over the `selected` range (it covers the emphasis). All are cut at each
/// other's edges, so the result is sorted and never overlaps, as gpui's text
/// highlights require. Ranges are clamped to the line's `length`.
pub fn code_highlights(
    length: usize,
    syntax: &[SyntaxSpan],
    changed: &[Range<usize>],
    emphasis: Rgba,
    selected: Option<&Range<usize>>,
) -> Vec<(Range<usize>, HighlightStyle)> {
    // Cut where the color, the change mark or the selection may switch,
    // then style each piece; plain, unchanged pieces need no highlight.
    let edges = syntax
        .iter()
        .map(|span| &span.range)
        .chain(changed)
        .chain(selected);
    edge_pieces(length, edges)
        .into_iter()
        .filter_map(|piece| {
            let span = syntax.iter().find(|span| covers(&span.range, &piece));
            let is_changed = changed.iter().any(|range| covers(range, &piece));
            let is_selected = selected.is_some_and(|range| covers(range, &piece));
            if span.is_none() && !is_changed && !is_selected {
                return None;
            }
            let mut highlight = HighlightStyle::default();
            if let Some(span) = span {
                highlight.color = Some(theme::syntax(span.kind).into());
            }
            if is_changed {
                highlight.background_color = Some(emphasis.into());
            }
            if is_selected {
                highlight.background_color = Some(theme::text_selection().into());
            }
            Some((piece, highlight))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use gpui::rgb;
    use qd_syntax::SyntaxKind;

    use super::*;

    fn span(range: Range<usize>) -> SyntaxSpan {
        SyntaxSpan {
            range,
            kind: SyntaxKind::Keyword,
        }
    }

    /// A change that starts inside a colored token splits it; both halves keep the color.
    #[test]
    fn change_splits_syntax_span() {
        let emphasis = rgb(0x00_33_00);
        let changed: [Range<usize>; 1] = [Range { start: 4, end: 8 }];
        let highlights = code_highlights(10, &[span(0..6)], &changed, emphasis, None);
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

    /// A selection splits the pieces it covers and draws over the change mark.
    #[test]
    fn selection_covers_change_mark() {
        let changed: [Range<usize>; 1] = [Range { start: 2, end: 6 }];
        let selected = Range { start: 4, end: 8 };
        let highlights = code_highlights(10, &[], &changed, rgb(0x00_33_00), Some(&selected));
        let selection: gpui::Hsla = theme::text_selection().into();
        let pieces: Vec<_> = highlights
            .iter()
            .map(|(range, style)| (range.clone(), style.background_color == Some(selection)))
            .collect();
        assert_eq!(pieces, [(2..4, false), (4..6, true), (6..8, true)]);
    }

    /// Without colors or changes there is nothing to highlight.
    #[test]
    fn plain_line_has_no_highlights() {
        assert_eq!(code_highlights(5, &[], &[], rgb(0), None).len(), 0);
    }
}
