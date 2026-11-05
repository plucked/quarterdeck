//! Syntax colors for file diffs, from syntect with bat's grammars (via
//! `two-face`) and the `TwoDark` theme. UI-free: spans carry plain RGB colors;
//! the app's `diff_view` turns them into text highlights.
//!
//! Files:
//! - `diff_syntax.rs`: `DiffSyntax`, the colors of every line of a `qc_diff::FileDiff`.
//! - `syntax_highlighter.rs`: the shared grammars and theme, language detection, line highlighting.
//! - `syntax_span.rs`: `SyntaxSpan`, a colored byte range of a line.
//! - `syntax_color.rs`: `SyntaxColor`, an RGB color.

mod diff_syntax;
mod syntax_color;
mod syntax_highlighter;
mod syntax_span;

pub use diff_syntax::DiffSyntax;
pub use syntax_color::SyntaxColor;
pub use syntax_span::SyntaxSpan;
