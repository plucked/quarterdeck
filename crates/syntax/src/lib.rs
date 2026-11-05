//! Syntax colors for file diffs, from syntect with bat's grammars (via
//! `two-face`) and the app's Graphite theme. UI-free: spans carry plain RGB colors;
//! the app's `diff_view` turns them into text highlights.
//!
//! Files:
//! - `diff_syntax.rs`: `DiffSyntax`, the colors of every line of a `qd_diff::FileDiff`.
//! - `syntax_highlighter.rs`: the shared grammars and theme, language detection, line highlighting.
//! - `graphite_theme.rs`: the Graphite syntax theme (seven colors by scope).
//! - `syntax_span.rs`: `SyntaxSpan`, a colored byte range of a line.
//! - `syntax_color.rs`: `SyntaxColor`, an RGB color.

mod diff_syntax;
mod graphite_theme;
mod syntax_color;
mod syntax_highlighter;
mod syntax_span;

pub use diff_syntax::DiffSyntax;
pub use syntax_color::SyntaxColor;
pub use syntax_span::SyntaxSpan;
