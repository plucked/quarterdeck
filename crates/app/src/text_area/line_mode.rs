use std::borrow::Cow;

/// Whether a text area holds many lines or one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineMode {
    /// Return inserts a newline; Up and Down move by visual row.
    Multiline,
    /// A field of one line, like a macOS text field: Return (or Enter on
    /// the keypad) submits, Up and Down go to the text's start and end, and
    /// newlines in pasted or input-method text are dropped. Long text still
    /// wraps on screen; it stays one logical line.
    SingleLine,
}

impl LineMode {
    /// `text` as it goes into an area of this mode: unchanged in a
    /// multiline area; without its newlines (`\n`, `\r\n` and a lone `\r`
    /// alike) in a single-line one. Dropped, not replaced by a space, since
    /// the one-line fields take values that hold no spaces (a token, a
    /// repository, a branch pattern), and a copied line often ends in one.
    pub fn inserted_text(self, text: &str) -> Cow<'_, str> {
        match self {
            Self::Multiline => Cow::Borrowed(text),
            Self::SingleLine => without_newlines(text),
        }
    }
}

/// `text` without its newlines; borrowed when it has none.
fn without_newlines(text: &str) -> Cow<'_, str> {
    if !text.contains(is_newline) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .filter(|character| !is_newline(*character))
            .collect(),
    )
}

/// Whether `character` ends a line: `\n`, or `\r` (alone or before `\n`).
fn is_newline(character: char) -> bool {
    matches!(character, '\n' | '\r')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A single-line area drops every kind of newline from inserted text.
    #[test]
    fn single_line_drops_newlines() {
        let mode = LineMode::SingleLine;
        assert_eq!(mode.inserted_text("owner/name\n"), "owner/name");
        assert_eq!(mode.inserted_text("ghp_abc\r\n"), "ghp_abc");
        assert_eq!(mode.inserted_text("feat/\rlegacy-*"), "feat/legacy-*");
        assert_eq!(mode.inserted_text("one\n\ntwo\r\nthree"), "onetwothree");
        assert_eq!(mode.inserted_text("\n"), "");
    }

    /// Text without newlines goes into a single-line area as it is, uncopied.
    #[test]
    fn single_line_keeps_one_line_text() {
        let inserted = LineMode::SingleLine.inserted_text("héllo wörld");
        assert!(matches!(inserted, Cow::Borrowed("héllo wörld")));
    }

    /// A multiline area takes inserted text unchanged, newlines included.
    #[test]
    fn multiline_keeps_newlines() {
        let inserted = LineMode::Multiline.inserted_text("one\ntwo\r\nthree");
        assert!(matches!(inserted, Cow::Borrowed("one\ntwo\r\nthree")));
    }
}
