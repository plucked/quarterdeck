use std::ops::Range;
use std::time::Duration;

use similar::{Algorithm, ChangeTag, TextDiffConfig};

use crate::{ChangeHighlight, WhitespaceMode};

/// Below this similarity a pair of lines counts as rewritten, not edited:
/// marking most of both lines would be noise.
const MIN_SIMILARITY: f32 = 0.4;
/// The longest a single line comparison may take before giving up on exactness.
const COMPARISON_TIMEOUT: Duration = Duration::from_millis(5);

/// The parts of a removed line and of the added line replacing it that differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineChanges {
    /// Byte ranges of the old (removed) line.
    pub old: Vec<Range<usize>>,
    /// Byte ranges of the new (added) line.
    pub new: Vec<Range<usize>>,
}

impl InlineChanges {
    /// Compares `old` and `new` by words or characters. `None` when only
    /// whole lines are highlighted, or when the lines are too different for
    /// a comparison to help.
    pub fn between(
        old: &str,
        new: &str,
        highlight: ChangeHighlight,
        whitespace: WhitespaceMode,
    ) -> Option<Self> {
        let mut config = TextDiffConfig::default();
        config
            .algorithm(Algorithm::Myers)
            .timeout(COMPARISON_TIMEOUT);
        let old_words = split_words(old);
        let new_words = split_words(new);
        let diff = match highlight {
            ChangeHighlight::Lines => return None,
            ChangeHighlight::Words => config.diff_slices(&old_words, &new_words),
            ChangeHighlight::Characters => config.diff_chars(old, new),
        };
        if diff.ratio() < MIN_SIMILARITY {
            return None;
        }

        // Walk the changes, tracking where each side is, and collect the differing pieces.
        let mut changes = Self {
            old: Vec::new(),
            new: Vec::new(),
        };
        let mut old_position = 0;
        let mut new_position = 0;
        for change in diff.iter_all_changes() {
            let length = change.value().len();
            let is_whitespace_only = change.value().trim().is_empty();
            let is_marked = whitespace == WhitespaceMode::Show || !is_whitespace_only;
            match change.tag() {
                ChangeTag::Equal => {
                    old_position += length;
                    new_position += length;
                }
                ChangeTag::Delete => {
                    if is_marked {
                        push_merged(&mut changes.old, old_position..old_position + length);
                    }
                    old_position += length;
                }
                ChangeTag::Insert => {
                    if is_marked {
                        push_merged(&mut changes.new, new_position..new_position + length);
                    }
                    new_position += length;
                }
            }
        }
        Some(changes)
    }
}

/// Splits a line into words as code reads: runs of letters, digits and `_`,
/// runs of whitespace, and each other character on its own, so `b = 2;`
/// changing to `b = 20;` marks `2`, not `2;`.
fn split_words(line: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start = 0;
    let mut previous_class = None;
    for (index, character) in line.char_indices() {
        let class = CharacterClass::of(character);
        let continues_word = previous_class == Some(class) && class != CharacterClass::Symbol;
        if !continues_word && index > start {
            words.push(&line[start..index]);
            start = index;
        }
        previous_class = Some(class);
    }
    if start < line.len() {
        words.push(&line[start..]);
    }
    words
}

/// What kind of word a character belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CharacterClass {
    Word,
    Whitespace,
    /// Punctuation and operators: each is a word of its own.
    Symbol,
}

impl CharacterClass {
    fn of(character: char) -> Self {
        if character.is_alphanumeric() || character == '_' {
            Self::Word
        } else if character.is_whitespace() {
            Self::Whitespace
        } else {
            Self::Symbol
        }
    }
}

/// Appends `range`, joining it to the previous range when they touch.
fn push_merged(ranges: &mut Vec<Range<usize>>, range: Range<usize>) {
    match ranges.last_mut() {
        Some(last) if last.end == range.start => last.end = range.end,
        Some(_) | None => ranges.push(range),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A changed word is marked on both sides, and nothing else.
    #[test]
    fn marks_changed_words() {
        let changes = InlineChanges::between(
            "let count = 1;",
            "let total = 1;",
            ChangeHighlight::Words,
            WhitespaceMode::Show,
        )
        .unwrap();
        assert_eq!(changes.old, vec![4..9]);
        assert_eq!(changes.new, vec![4..9]);
    }

    /// Character mode marks only the characters that differ.
    #[test]
    fn marks_changed_characters() {
        let changes = InlineChanges::between(
            "color",
            "colour",
            ChangeHighlight::Characters,
            WhitespaceMode::Show,
        )
        .unwrap();
        assert_eq!(changes.old, Vec::<Range<usize>>::new());
        assert_eq!(changes.new, vec![4..5]);
    }

    /// Words split at punctuation, and each symbol stands alone.
    #[test]
    fn splits_words_at_punctuation() {
        assert_eq!(split_words("b = 20;"), ["b", " ", "=", " ", "20", ";"]);
        assert_eq!(split_words("a::b()"), ["a", ":", ":", "b", "(", ")"]);
    }

    /// Lines that share little are not compared.
    #[test]
    fn rewritten_lines_are_not_compared() {
        assert_eq!(
            InlineChanges::between(
                "fn alpha() {}",
                "struct Zeta;",
                ChangeHighlight::Words,
                WhitespaceMode::Show
            ),
            None
        );
    }

    /// Ignoring whitespace leaves whitespace-only differences unmarked.
    #[test]
    fn ignores_whitespace_pieces() {
        let changes = InlineChanges::between(
            "a b c",
            "a  b d",
            ChangeHighlight::Words,
            WhitespaceMode::Ignore,
        )
        .unwrap();
        assert!(
            changes
                .new
                .iter()
                .all(|range| !"a  b d"[range.clone()].trim().is_empty())
        );
    }
}
