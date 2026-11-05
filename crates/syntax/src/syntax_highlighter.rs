use std::path::Path;
use std::sync::OnceLock;

use crate::graphite_theme::graphite_theme;
use crate::{SyntaxColor, SyntaxSpan};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, Style, Theme};
use syntect::parsing::{SyntaxReference, SyntaxSet};

/// Lines longer than this (minified code, data) are left plain: they are
/// slow to highlight and unreadable either way.
const MAX_HIGHLIGHTED_LINE_LENGTH: usize = 2_000;

/// bat's grammars and the theme, loaded once on first use (it takes a
/// moment, so first use should be on the background executor).
pub(crate) struct SyntaxHighlighter {
    syntaxes: SyntaxSet,
    theme: Theme,
}

impl SyntaxHighlighter {
    /// The shared highlighter, loading it on first call.
    pub fn shared() -> &'static Self {
        static SHARED: OnceLock<SyntaxHighlighter> = OnceLock::new();
        SHARED.get_or_init(|| Self {
            syntaxes: two_face::syntax::extra_newlines(),
            theme: graphite_theme(),
        })
    }

    /// The grammar for `path`: by extension, then by whole file name
    /// (`Dockerfile`, `Makefile`), then by `first_line` (a shebang).
    pub fn find_syntax(&self, path: &str, first_line: Option<&str>) -> Option<&SyntaxReference> {
        let path = Path::new(path);
        let by_extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .and_then(|extension| self.syntaxes.find_syntax_by_extension(extension));
        let by_file_name = || {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| self.syntaxes.find_syntax_by_extension(name))
        };
        let by_first_line =
            || first_line.and_then(|line| self.syntaxes.find_syntax_by_first_line(line));
        by_extension
            .or_else(by_file_name)
            .or_else(by_first_line)
            .filter(|syntax| syntax.name != "Plain Text")
    }

    /// Highlights consecutive `lines` of one file with `syntax`, carrying the
    /// parser state from line to line. One result per line; a line that
    /// fails to parse or is too long gets no spans.
    pub fn highlight_lines<'line>(
        &self,
        syntax: &SyntaxReference,
        lines: impl Iterator<Item = &'line str>,
    ) -> Vec<Vec<SyntaxSpan>> {
        let mut highlighter = HighlightLines::new(syntax, &self.theme);
        lines
            .map(|line| {
                if line.len() > MAX_HIGHLIGHTED_LINE_LENGTH {
                    return Vec::new();
                }
                // The grammars expect each line with its newline.
                let with_newline = format!("{line}\n");
                highlighter
                    .highlight_line(&with_newline, &self.syntaxes)
                    .map(|pieces| self.spans_for(&pieces, line.len()))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Turns highlighted pieces into spans within `line_length`, leaving out
    /// pieces in the default style.
    fn spans_for(&self, pieces: &[(Style, &str)], line_length: usize) -> Vec<SyntaxSpan> {
        let default_foreground = self.theme.settings.foreground;
        let mut spans = Vec::new();
        let mut position = 0;
        for (style, text) in pieces {
            let start = position;
            position += text.len();
            let range = start..position.min(line_length);
            let is_bold = style.font_style.contains(FontStyle::BOLD);
            let is_italic = style.font_style.contains(FontStyle::ITALIC);
            let is_default = Some(style.foreground) == default_foreground && !is_bold && !is_italic;
            if range.is_empty() || is_default {
                continue;
            }
            spans.push(SyntaxSpan {
                range,
                color: syntax_color(style.foreground),
                is_bold,
                is_italic,
            });
        }
        spans
    }
}

fn syntax_color(color: Color) -> SyntaxColor {
    SyntaxColor {
        red: color.r,
        green: color.g,
        blue: color.b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Grammars are found by extension, by file name and by shebang.
    #[test]
    fn finds_syntax_by_path_and_shebang() {
        let highlighter = SyntaxHighlighter::shared();
        let name = |path, first_line| {
            highlighter
                .find_syntax(path, first_line)
                .map(|syntax| syntax.name.clone())
        };
        assert_eq!(name("src/main.rs", None).as_deref(), Some("Rust"));
        assert_eq!(name("web/app.ts", None).as_deref(), Some("TypeScript"));
        assert_eq!(name("Assets/Player.cs", None).as_deref(), Some("C#"));
        assert_eq!(name("Dockerfile", None).as_deref(), Some("Dockerfile"));
        assert_eq!(
            name("bin/run", Some("#!/usr/bin/env python3")).as_deref(),
            Some("Python")
        );
        assert_eq!(name("notes.unknown-ext", None), None);
    }

    /// Keywords get a color; the parser state carries into the next line.
    #[test]
    fn highlights_lines_with_state() {
        let highlighter = SyntaxHighlighter::shared();
        let rust = highlighter.find_syntax("a.rs", None).unwrap();
        let spans = highlighter.highlight_lines(
            rust,
            ["fn main() {", "/* open", "still comment */"].into_iter(),
        );
        assert!(spans[0].iter().any(|span| span.range == (0..2)));
        // The second comment line is colored like the first, all of it.
        let comment_color = spans[1][0].color;
        assert_eq!(spans[2][0].color, comment_color);
    }
}
