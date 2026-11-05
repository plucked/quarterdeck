use gpui::{Div, FontWeight, IntoElement, Rgba, div, prelude::*, px};
use qd_cache::ColorTheme;
use qd_syntax::SyntaxKind;

use crate::theme::{self, ThemeColors, colors_of, palette_of};

/// Height of one code line in the preview, as in the diff.
const LINE_HEIGHT: f32 = 18.;
/// Width of each line-number column.
const GUTTER_WIDTH: f32 = 30.;
const MARKER_WIDTH: f32 = 16.;

/// How a preview line differs between the two sides.
#[derive(Clone, Copy)]
enum PreviewLine {
    Context,
    Removed,
    Added,
}

/// One piece of a preview line.
struct Piece {
    text: &'static str,
    /// The token kind; `None` for plain text.
    token: Option<SyntaxKind>,
    /// Whether it is a changed word (on the stronger background).
    is_changed: bool,
}

/// One line of the preview's hunk.
struct SampleLine {
    old: Option<u32>,
    new: Option<u32>,
    change: PreviewLine,
    pieces: &'static [Piece],
}

/// A hunk of Rust as the diff draws it.
const SAMPLE: [SampleLine; 7] = [
    SampleLine {
        old: Some(12),
        new: Some(12),
        change: PreviewLine::Context,
        pieces: &[Piece {
            text: "// WCAG 2 contrast of two colors, 1 to 21",
            token: Some(SyntaxKind::Comment),
            is_changed: false,
        }],
    },
    SampleLine {
        old: Some(13),
        new: Some(13),
        change: PreviewLine::Context,
        pieces: &[
            Piece {
                text: "pub fn ",
                token: Some(SyntaxKind::Keyword),
                is_changed: false,
            },
            Piece {
                text: "contrast_ratio",
                token: Some(SyntaxKind::Definition),
                is_changed: false,
            },
            Piece {
                text: "(text: ",
                token: None,
                is_changed: false,
            },
            Piece {
                text: "Rgb",
                token: Some(SyntaxKind::Type),
                is_changed: false,
            },
            Piece {
                text: ", ground: ",
                token: None,
                is_changed: false,
            },
            Piece {
                text: "Rgb",
                token: Some(SyntaxKind::Type),
                is_changed: false,
            },
            Piece {
                text: ") -> ",
                token: None,
                is_changed: false,
            },
            Piece {
                text: "f32",
                token: Some(SyntaxKind::Keyword),
                is_changed: false,
            },
            Piece {
                text: " {",
                token: None,
                is_changed: false,
            },
        ],
    },
    SampleLine {
        old: Some(14),
        new: Some(14),
        change: PreviewLine::Context,
        pieces: &[
            Piece {
                text: "    let ",
                token: Some(SyntaxKind::Keyword),
                is_changed: false,
            },
            Piece {
                text: "(hi, lo) = ",
                token: None,
                is_changed: false,
            },
            Piece {
                text: "sorted",
                token: Some(SyntaxKind::Definition),
                is_changed: false,
            },
            Piece {
                text: "(text, ground);",
                token: None,
                is_changed: false,
            },
        ],
    },
    SampleLine {
        old: Some(15),
        new: None,
        change: PreviewLine::Removed,
        pieces: &[
            Piece {
                text: "    ",
                token: None,
                is_changed: false,
            },
            Piece {
                text: "(hi + ",
                token: None,
                is_changed: true,
            },
            Piece {
                text: "0.05",
                token: Some(SyntaxKind::Constant),
                is_changed: true,
            },
            Piece {
                text: ") / (lo + ",
                token: None,
                is_changed: true,
            },
            Piece {
                text: "0.05",
                token: Some(SyntaxKind::Constant),
                is_changed: true,
            },
            Piece {
                text: ")",
                token: None,
                is_changed: true,
            },
        ],
    },
    SampleLine {
        old: None,
        new: Some(15),
        change: PreviewLine::Added,
        pieces: &[
            Piece {
                text: "    let ",
                token: Some(SyntaxKind::Keyword),
                is_changed: false,
            },
            Piece {
                text: "ratio = (hi + ",
                token: None,
                is_changed: false,
            },
            Piece {
                text: "0.05",
                token: Some(SyntaxKind::Constant),
                is_changed: false,
            },
            Piece {
                text: ") / (lo + ",
                token: None,
                is_changed: false,
            },
            Piece {
                text: "0.05",
                token: Some(SyntaxKind::Constant),
                is_changed: false,
            },
            Piece {
                text: ");",
                token: None,
                is_changed: false,
            },
        ],
    },
    SampleLine {
        old: None,
        new: Some(16),
        change: PreviewLine::Added,
        pieces: &[
            Piece {
                text: "    ",
                token: None,
                is_changed: false,
            },
            Piece {
                text: "ensure!",
                token: Some(SyntaxKind::Definition),
                is_changed: true,
            },
            Piece {
                text: "(ratio >= ",
                token: None,
                is_changed: true,
            },
            Piece {
                text: "1.0",
                token: Some(SyntaxKind::Constant),
                is_changed: true,
            },
            Piece {
                text: ", ",
                token: None,
                is_changed: true,
            },
            Piece {
                text: "\"ratio below 1\"",
                token: Some(SyntaxKind::String),
                is_changed: true,
            },
            Piece {
                text: ");",
                token: None,
                is_changed: true,
            },
        ],
    },
    SampleLine {
        old: Some(16),
        new: Some(17),
        change: PreviewLine::Context,
        pieces: &[Piece {
            text: "}",
            token: None,
            is_changed: false,
        }],
    },
];

/// A small diff drawn in `theme`'s colors, whether or not it is active:
/// a pull request header (title, metadata, a primary button), then a hunk
/// of code with syntax colors, line numbers and changed lines.
pub fn render_theme_preview(color_theme: ColorTheme) -> impl IntoElement {
    let colors = colors_of(color_theme);
    let palette = palette_of(color_theme);
    div()
        .flex()
        .flex_col()
        .rounded_lg()
        .overflow_hidden()
        .border_1()
        .border_color(colors.border)
        .bg(colors.background)
        .text_color(colors.text)
        .child(render_preview_header(colors, palette.name))
        .child(render_hunk_header(colors))
        .child(
            div()
                .flex()
                .flex_col()
                .pb_2()
                .font_family(theme::MONO_FONT)
                .text_xs()
                .children(SAMPLE.iter().map(|line| render_preview_line(colors, line))),
        )
}

/// The pull request header: title, metadata and a primary button.
fn render_preview_header(colors: &ThemeColors, theme_name: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_3()
        .py_2()
        .bg(colors.panel)
        .border_b_1()
        .border_color(colors.border)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .truncate()
                        .child(format!("Round contrast ratios · {theme_name}")),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(colors.muted)
                        .flex()
                        .gap_1()
                        .child("#4821 · octocat · 2 h")
                        .child(div().text_color(colors.added).child("+2"))
                        .child(div().text_color(colors.removed).child("−1")),
                ),
        )
        .child(
            div()
                .flex_none()
                .px_2()
                .py_0p5()
                .rounded_md()
                .text_xs()
                .bg(colors.accent)
                .text_color(colors.on_accent)
                .child("Approve"),
        )
}

/// The `@@ … @@` row above the code.
fn render_hunk_header(colors: &ThemeColors) -> Div {
    div()
        .h(px(LINE_HEIGHT + 4.))
        .flex()
        .items_center()
        .px_3()
        .bg(colors.hunk_background)
        .text_color(colors.subtle)
        .font_family(theme::MONO_FONT)
        .text_xs()
        .child("@@ -12,5 +12,6 @@ fn contrast_ratio")
}

/// One code line: two line numbers, the change mark, the colored pieces.
fn render_preview_line(colors: &ThemeColors, line: &SampleLine) -> Div {
    let (background, mark, mark_color, emphasis) = match line.change {
        PreviewLine::Context => (colors.background, "", colors.muted, colors.background),
        PreviewLine::Removed => (
            colors.removed_background,
            "-",
            colors.removed,
            colors.removed_emphasis,
        ),
        PreviewLine::Added => (
            colors.added_background,
            "+",
            colors.added,
            colors.added_emphasis,
        ),
    };
    let gutter = |number: Option<u32>| {
        div()
            .flex_none()
            .w(px(GUTTER_WIDTH))
            .pr_1()
            .text_right()
            .text_color(colors.line_number)
            .children(number.map(|number| number.to_string()))
    };
    div()
        .h(px(LINE_HEIGHT))
        .flex()
        .items_center()
        .whitespace_nowrap()
        .overflow_hidden()
        .bg(background)
        .child(gutter(line.old))
        .child(gutter(line.new))
        .child(
            div()
                .flex_none()
                .w(px(MARKER_WIDTH))
                .text_center()
                .text_color(mark_color)
                .child(mark),
        )
        .children(line.pieces.iter().map(|piece| {
            let color: Rgba = piece.token.map_or(colors.text, |kind| colors.syntax(kind));
            div()
                .text_color(color)
                .when(piece.is_changed, |element| element.bg(emphasis))
                .child(piece.text)
        }))
}
