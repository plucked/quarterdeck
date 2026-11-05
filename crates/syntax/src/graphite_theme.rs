use std::str::FromStr as _;

use syntect::highlighting::{
    Color, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};

/// Plain code: the app's text color.
const FOREGROUND: u32 = 0xe5_e3_df;
const BACKGROUND: u32 = 0x1c_1e_21;
/// Every syntax color clears 4.5:1 on the background (APCA |Lc| 60 or more);
/// comments stay readable rather than faded, since they are content.
const COMMENT: u32 = 0xbe_b8_9f;
const KEYWORD: u32 = 0xde_c3_ff;
const STRING: u32 = 0x8c_e1_99;
const CONSTANT: u32 = 0xff_c2_8c;
const DEFINITION: u32 = 0xae_d1_fe;
const TYPE: u32 = 0x3d_e3_e3;
const INVALID: u32 = 0xfe_bf_b7;

/// Which scopes get which color: Sublime Text scope selectors as bat's grammars
/// name them. Variables, operators and punctuation keep the foreground.
const SCOPE_COLORS: [(&str, u32); 7] = [
    ("comment, punctuation.definition.comment", COMMENT),
    (
        "keyword - keyword.operator, keyword.operator.word, storage",
        KEYWORD,
    ),
    (
        "string, string punctuation.definition.string, constant.character.escape",
        STRING,
    ),
    (
        "constant.numeric, constant.language, constant.other, constant.character",
        CONSTANT,
    ),
    (
        "entity.name.function, support.function, meta.function-call entity.name, entity.name.method",
        DEFINITION,
    ),
    (
        "entity.name.type, entity.name.class, entity.name.struct, entity.name.enum, entity.name.trait, support.type, support.class, storage.type.primitive",
        TYPE,
    ),
    ("invalid", INVALID),
];

/// The Graphite syntax theme: seven colors on the app's dark grey, from
/// the Proofreader Palettes' Graphite (dark) palette.
pub(crate) fn graphite_theme() -> Theme {
    Theme {
        name: Some("Graphite".to_owned()),
        author: None,
        settings: ThemeSettings {
            foreground: Some(color(FOREGROUND)),
            background: Some(color(BACKGROUND)),
            ..ThemeSettings::default()
        },
        scopes: SCOPE_COLORS
            .iter()
            .map(|(selectors, rgb)| ThemeItem {
                scope: ScopeSelectors::from_str(selectors)
                    .expect("the theme's scope selectors are valid"),
                style: StyleModifier {
                    foreground: Some(color(*rgb)),
                    background: None,
                    font_style: None,
                },
            })
            .collect(),
    }
}

/// An opaque color from `0xRRGGBB`.
fn color(rgb: u32) -> Color {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color { r, g, b, a: 0xff }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every selector parses, so the theme never panics on first use.
    #[test]
    fn builds_every_scope() {
        assert_eq!(graphite_theme().scopes.len(), SCOPE_COLORS.len());
    }

    /// bat's Rust grammar gets the intended colors: keyword, definition,
    /// type, string, number and comment; operators keep the foreground.
    #[test]
    fn colors_rust_tokens() {
        let highlighter = crate::syntax_highlighter::SyntaxHighlighter::shared();
        let syntax = highlighter.find_syntax("main.rs", None).unwrap();
        let line = r#"fn parse(names: Vec<u8>) { let x = "x"; 42 } // done"#;
        let spans = highlighter.highlight_lines(syntax, std::iter::once(line));
        let color_of = |text: &str| {
            let start = line.find(text).unwrap();
            let span = spans[0]
                .iter()
                .find(|span| span.range.contains(&start))
                .unwrap_or_else(|| panic!("{text} has no color"));
            let rgb = span.color;
            u32::from_be_bytes([0, rgb.red, rgb.green, rgb.blue])
        };
        assert_eq!(color_of("fn"), KEYWORD);
        assert_eq!(color_of("parse"), DEFINITION);
        assert_eq!(color_of("Vec"), TYPE);
        assert_eq!(color_of("let"), KEYWORD);
        // Operators stay plain: no span at all.
        let equals = line.find('=').unwrap();
        assert!(!spans[0].iter().any(|span| span.range.contains(&equals)));
        assert_eq!(color_of("\"x\""), STRING);
        assert_eq!(color_of("42"), CONSTANT);
        assert_eq!(color_of("// done"), COMMENT);
    }
}
