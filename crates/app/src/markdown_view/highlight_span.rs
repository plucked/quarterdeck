use std::ops::Range;

use qc_markdown::{InlineStyle, StyledRun};

/// A byte range of a text with one Markdown style and one selection state:
/// the pieces a text's styled runs and the selection cut it into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub range: Range<usize>,
    pub style: InlineStyle,
    pub is_selected: bool,
}

impl HighlightSpan {
    /// Cuts a text of `length` bytes at every run edge and at the selection's
    /// edges, so each piece has one style and is either wholly selected or
    /// not. Plain, unselected pieces are left out; the result is sorted and
    /// never overlaps, as gpui's text highlights require.
    pub fn split(length: usize, runs: &[StyledRun], selected: Option<&Range<usize>>) -> Vec<Self> {
        // Every place where the style or the selection may change.
        let mut edges = vec![0, length];
        for run in runs {
            edges.extend([run.range.start, run.range.end]);
        }
        if let Some(selected) = selected {
            edges.extend([selected.start.min(length), selected.end.min(length)]);
        }
        edges.sort_unstable();
        edges.dedup();

        // Describe each piece between two edges and keep the ones that need a highlight.
        edges
            .windows(2)
            .filter_map(|pair| {
                let range = pair[0]..pair[1];
                let style = runs
                    .iter()
                    .find(|run| run.range.start <= range.start && range.end <= run.range.end)
                    .map_or(InlineStyle::PLAIN, |run| run.style);
                let is_selected = selected.is_some_and(|selected| {
                    selected.start <= range.start && range.end <= selected.end
                });
                let needs_highlight = !style.is_plain() || is_selected;
                needs_highlight.then_some(Self {
                    range,
                    style,
                    is_selected,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(range: Range<usize>, style: InlineStyle) -> StyledRun {
        StyledRun { range, style }
    }

    /// Without a selection the spans are exactly the styled runs.
    #[test]
    fn spans_follow_runs_without_selection() {
        let runs = [run(2..5, InlineStyle::STRONG)];
        assert_eq!(
            HighlightSpan::split(8, &runs, None),
            [HighlightSpan {
                range: 2..5,
                style: InlineStyle::STRONG,
                is_selected: false
            }]
        );
    }

    /// A selection that crosses a run's edge splits the run.
    #[test]
    fn selection_splits_runs() {
        let runs = [run(2..6, InlineStyle::CODE)];
        let spans = HighlightSpan::split(10, &runs, Some(&(4..8)));
        let pieces: Vec<_> = spans
            .iter()
            .map(|span| (span.range.clone(), span.style, span.is_selected))
            .collect();
        assert_eq!(
            pieces,
            [
                (2..4, InlineStyle::CODE, false),
                (4..6, InlineStyle::CODE, true),
                (6..8, InlineStyle::PLAIN, true),
            ]
        );
    }

    /// A selection past the end of the text is clamped to it.
    #[test]
    fn selection_is_clamped_to_text() {
        let spans = HighlightSpan::split(3, &[], Some(&(1..99)));
        assert_eq!(spans[0].range, 1..3);
        assert_eq!(spans.len(), 1);
    }
}
