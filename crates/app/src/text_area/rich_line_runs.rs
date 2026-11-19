use std::ops::Range;

use qd_markdown::{InlineStyle, RichDocument};

/// Maps nested GitHub Markdown styles onto the same measured glyph runs used for editing.
pub(super) fn rich_line_runs(
    document: &RichDocument,
    line: Range<usize>,
    base: &gpui::TextRun,
) -> Vec<gpui::TextRun> {
    let mut runs = Vec::new();
    let mut cursor = line.start;
    let first = document
        .runs
        .partition_point(|run| run.range.end <= line.start);
    for styled in &document.runs[first..] {
        if styled.range.start >= line.end {
            break;
        }
        let start = styled.range.start.max(line.start);
        let end = styled.range.end.min(line.end);
        if start > cursor {
            runs.push(gpui::TextRun {
                len: start - cursor,
                ..base.clone()
            });
        }
        let highlight = crate::markdown_view::inline_highlight(styled.style, false);
        let mut run = base.clone();
        run.len = end - start;
        if let Some(weight) = highlight.font_weight {
            run.font.weight = weight;
        }
        if let Some(style) = highlight.font_style {
            run.font.style = style;
        }
        if styled.style.contains(InlineStyle::CODE) {
            run.font.family = crate::theme::MONO_FONT.into();
        }
        run.color = highlight.color.unwrap_or(base.color);
        run.underline = highlight.underline;
        run.strikethrough = highlight.strikethrough;
        run.background_color = highlight.background_color;
        runs.push(run);
        cursor = end;
    }
    if cursor < line.end || runs.is_empty() {
        runs.push(gpui::TextRun {
            len: line.end - cursor,
            ..base.clone()
        });
    }
    runs
}
