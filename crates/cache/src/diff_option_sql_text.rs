//! SQL text for the `qd-diff` enums stored in `diff_view_settings`. They are
//! mapped here, beside their store, because `qd-diff` knows nothing of SQLite.

use qd_diff::{ChangeHighlight, WhitespaceMode};

const HIGHLIGHTS: [ChangeHighlight; 3] = [
    ChangeHighlight::Lines,
    ChangeHighlight::Words,
    ChangeHighlight::Characters,
];
const WHITESPACE_MODES: [WhitespaceMode; 2] = [WhitespaceMode::Show, WhitespaceMode::Ignore];

/// The stored value of a highlight; the table's CHECK constraint lists the same values.
pub(crate) fn highlight_as_sql_text(highlight: ChangeHighlight) -> &'static str {
    match highlight {
        ChangeHighlight::Lines => "lines",
        ChangeHighlight::Words => "words",
        ChangeHighlight::Characters => "characters",
    }
}

/// Decodes a stored highlight; `None` for anything else.
pub(crate) fn highlight_from_sql_text(text: &str) -> Option<ChangeHighlight> {
    HIGHLIGHTS
        .into_iter()
        .find(|highlight| highlight_as_sql_text(*highlight) == text)
}

/// The stored value of a whitespace mode; the table's CHECK constraint lists the same values.
pub(crate) fn whitespace_as_sql_text(whitespace: WhitespaceMode) -> &'static str {
    match whitespace {
        WhitespaceMode::Show => "show",
        WhitespaceMode::Ignore => "ignore",
    }
}

/// Decodes a stored whitespace mode; `None` for anything else.
pub(crate) fn whitespace_from_sql_text(text: &str) -> Option<WhitespaceMode> {
    WHITESPACE_MODES
        .into_iter()
        .find(|whitespace| whitespace_as_sql_text(*whitespace) == text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every highlight and whitespace mode decodes from the text it is stored as.
    #[test]
    fn options_round_trip_through_sql_text() {
        for highlight in HIGHLIGHTS {
            assert_eq!(
                highlight_from_sql_text(highlight_as_sql_text(highlight)),
                Some(highlight)
            );
        }
        for whitespace in WHITESPACE_MODES {
            assert_eq!(
                whitespace_from_sql_text(whitespace_as_sql_text(whitespace)),
                Some(whitespace)
            );
        }
    }
}
