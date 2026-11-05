use std::ops::Range;

/// The word around byte `index` of `text`, for a double click: the run of
/// letters, digits and `_` it touches, or the single character there when it
/// is none of those. Empty at the end of the text.
pub fn word_range_at(text: &str, index: usize) -> Range<usize> {
    let is_word_character = |character: char| character.is_alphanumeric() || character == '_';
    let index = floor_char_boundary(text, index.min(text.len()));

    // A non-word character (space, punctuation) is selected on its own.
    let Some(character) = text[index..].chars().next() else {
        return index..index;
    };
    if !is_word_character(character) {
        return index..index + character.len_utf8();
    }

    // Grow the range over word characters on both sides.
    let start = text[..index]
        .char_indices()
        .rev()
        .take_while(|(_, character)| is_word_character(*character))
        .last()
        .map_or(index, |(position, _)| position);
    let end = text[index..]
        .char_indices()
        .find(|(_, character)| !is_word_character(*character))
        .map_or(text.len(), |(offset, _)| index + offset);
    start..end
}

/// The nearest char boundary at or before `index`.
fn floor_char_boundary(text: &str, index: usize) -> usize {
    (0..=index)
        .rev()
        .find(|position| text.is_char_boundary(*position))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A click inside a word selects the whole word.
    #[test]
    fn selects_word_around_index() {
        let text = "fix the review_queue now";
        assert_eq!(&text[word_range_at(text, 10)], "review_queue");
    }

    /// A click on a space selects just that space.
    #[test]
    fn selects_single_separator() {
        assert_eq!(word_range_at("a b", 1), 1..2);
    }

    /// Multi-byte characters keep the range on char boundaries.
    #[test]
    fn handles_multibyte_text() {
        let text = "größe ok";
        assert_eq!(&text[word_range_at(text, 3)], "größe");
    }

    /// The end of the text selects nothing.
    #[test]
    fn end_of_text_is_empty() {
        assert_eq!(word_range_at("abc", 3), 3..3);
    }
}
