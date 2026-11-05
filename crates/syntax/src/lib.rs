//! Syntax colors for file diffs, from syntect with bat's grammars (via
//! `two-face`) sorted into token kinds. UI-free: spans carry a `SyntaxKind`; the app's
//! `diff_view` colors each kind from the active color theme.
//!
//! Files:
//! - `diff_syntax.rs`: `DiffSyntax`, the colors of every line of a `qd_diff::FileDiff`.
//! - `syntax_highlighter.rs`: the shared grammars and theme, language detection, line highlighting.
//! - `token_theme.rs`: the syntect theme that sorts scopes into `SyntaxKind`s.
//! - `syntax_span.rs`: `SyntaxSpan`, the byte range of one token kind in a line.
//! - `syntax_kind.rs`: `SyntaxKind`, what a token is (comment, keyword, string, …).

mod diff_syntax;
mod syntax_highlighter;
mod syntax_kind;
mod syntax_span;
mod token_theme;

pub use diff_syntax::DiffSyntax;
pub use syntax_kind::SyntaxKind;
pub use syntax_span::SyntaxSpan;
