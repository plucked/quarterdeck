use std::str::FromStr as _;

use syntect::highlighting::{
    Color, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};

use crate::SyntaxKind;

/// Which scopes are which kind of token: Sublime Text scope selectors as
/// bat's grammars name them. Variables, operators and punctuation stay plain.
const SCOPE_KINDS: [(&str, SyntaxKind); 7] = [
    (
        "comment, punctuation.definition.comment",
        SyntaxKind::Comment,
    ),
    (
        "keyword - keyword.operator, keyword.operator.word, storage",
        SyntaxKind::Keyword,
    ),
    (
        "string, string punctuation.definition.string, constant.character.escape",
        SyntaxKind::String,
    ),
    (
        "constant.numeric, constant.language, constant.other, constant.character",
        SyntaxKind::Constant,
    ),
    (
        "entity.name.function, support.function, meta.function-call entity.name, entity.name.method",
        SyntaxKind::Definition,
    ),
    (
        "entity.name.type, entity.name.class, entity.name.struct, entity.name.enum, entity.name.trait, support.type, support.class, storage.type.primitive",
        SyntaxKind::Type,
    ),
    ("invalid", SyntaxKind::Invalid),
];

/// A syntect theme that sorts tokens instead of coloring them: each kind
/// gets a marker color (its position, 1 to 7, in the red channel) that
/// `kind_of_marker` turns back into the kind; plain text gets black.
pub(crate) fn token_theme() -> Theme {
    Theme {
        name: Some("Token kinds".to_owned()),
        author: None,
        settings: ThemeSettings {
            foreground: Some(PLAIN_MARKER),
            ..ThemeSettings::default()
        },
        scopes: SCOPE_KINDS
            .iter()
            .map(|(selectors, kind)| ThemeItem {
                scope: ScopeSelectors::from_str(selectors)
                    .expect("the theme's scope selectors are valid"),
                style: StyleModifier {
                    foreground: Some(marker_of(*kind)),
                    background: None,
                    font_style: None,
                },
            })
            .collect(),
    }
}

/// The marker of plain text.
const PLAIN_MARKER: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0xff,
};

/// The marker color standing for `kind`.
fn marker_of(kind: SyntaxKind) -> Color {
    let position = SyntaxKind::ALL
        .iter()
        .position(|candidate| *candidate == kind)
        .expect("every kind is in SyntaxKind::ALL");
    Color {
        r: u8::try_from(position + 1).expect("seven kinds fit a byte"),
        ..PLAIN_MARKER
    }
}

/// The kind a highlighted piece's marker color stands for; `None` for plain text.
pub(crate) fn kind_of_marker(color: Color) -> Option<SyntaxKind> {
    SyntaxKind::ALL
        .into_iter()
        .find(|kind| marker_of(*kind) == color)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every selector parses, and every kind's marker reads back as that kind.
    #[test]
    fn builds_every_scope() {
        assert_eq!(token_theme().scopes.len(), SCOPE_KINDS.len());
        for kind in SyntaxKind::ALL {
            assert_eq!(kind_of_marker(marker_of(kind)), Some(kind));
        }
        assert_eq!(kind_of_marker(PLAIN_MARKER), None);
    }

    /// bat's Rust grammar sorts tokens as intended; operators stay plain.
    #[test]
    fn sorts_rust_tokens() {
        let highlighter = crate::syntax_highlighter::SyntaxHighlighter::shared();
        let syntax = highlighter.find_syntax("main.rs", None).unwrap();
        let line = r#"fn parse(names: Vec<u8>) { let x = "x"; 42 } // done"#;
        let spans = highlighter.highlight_lines(syntax, std::iter::once(line));
        let kind_at = |text: &str| {
            let start = line.find(text).unwrap();
            spans[0]
                .iter()
                .find(|span| span.range.contains(&start))
                .map(|span| span.kind)
        };
        assert_eq!(kind_at("fn"), Some(SyntaxKind::Keyword));
        assert_eq!(kind_at("parse"), Some(SyntaxKind::Definition));
        assert_eq!(kind_at("Vec"), Some(SyntaxKind::Type));
        assert_eq!(kind_at("let"), Some(SyntaxKind::Keyword));
        assert_eq!(kind_at("=",), None);
        assert_eq!(kind_at("\"x\""), Some(SyntaxKind::String));
        assert_eq!(kind_at("42"), Some(SyntaxKind::Constant));
        assert_eq!(kind_at("// done"), Some(SyntaxKind::Comment));
    }
}
