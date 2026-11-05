use std::ops::Range;

/// Whether `character` is part of a word for word movement and double clicks.
fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// The character boundary before `offset` in `text`.
pub fn previous_boundary(text: &str, offset: usize) -> usize {
    text[..offset]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

/// The character boundary after `offset` in `text`.
pub fn next_boundary(text: &str, offset: usize) -> usize {
    text[offset..]
        .chars()
        .next()
        .map_or(text.len(), |next| offset + next.len_utf8())
}

/// The start of the word before `offset`, skipping what is between first.
pub fn previous_word_start(text: &str, offset: usize) -> usize {
    let mut characters = text[..offset].char_indices().rev().peekable();
    while characters
        .next_if(|(_, character)| !is_word_character(*character))
        .is_some()
    {}
    let mut start = characters.peek().map_or(0, |(index, _)| *index);
    while let Some((index, _)) = characters.next_if(|(_, character)| is_word_character(*character))
    {
        start = index;
    }
    start.min(offset)
}

/// The end of the word after `offset`, skipping what is between first.
pub fn next_word_end(text: &str, offset: usize) -> usize {
    let mut characters = text[offset..].char_indices().peekable();
    while characters
        .next_if(|(_, character)| !is_word_character(*character))
        .is_some()
    {}
    while characters
        .next_if(|(_, character)| is_word_character(*character))
        .is_some()
    {}
    characters
        .peek()
        .map_or(text.len(), |(index, _)| offset + index)
}

/// The start of the logical line holding `offset`.
pub fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |index| index + 1)
}

/// The end of the logical line holding `offset` (before its `\n`).
pub fn line_end(text: &str, offset: usize) -> usize {
    text[offset..]
        .find('\n')
        .map_or(text.len(), |index| offset + index)
}

/// The word around `offset`, for a double click; empty between words.
pub fn word_range_at(text: &str, offset: usize) -> Range<usize> {
    let start = text[..offset]
        .char_indices()
        .rev()
        .take_while(|(_, character)| is_word_character(*character))
        .last()
        .map_or(offset, |(index, _)| index);
    let end = text[offset..]
        .char_indices()
        .find(|(_, character)| !is_word_character(*character))
        .map_or(text.len(), |(index, _)| offset + index);
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Word movement skips spaces and punctuation and stops at word edges.
    #[test]
    fn word_boundaries() {
        let text = "fix the  bug_now.";
        assert_eq!(previous_word_start(text, 17), 9);
        assert_eq!(previous_word_start(text, 9), 4);
        assert_eq!(previous_word_start(text, 2), 0);
        assert_eq!(next_word_end(text, 0), 3);
        assert_eq!(next_word_end(text, 3), 7);
        assert_eq!(next_word_end(text, 16), 17);
        assert_eq!(word_range_at(text, 11), 9..16);
    }

    /// Line start and end stop at the newlines around an offset.
    #[test]
    fn line_boundaries() {
        let text = "one\ntwo\nthree";
        assert_eq!(line_start(text, 5), 4);
        assert_eq!(line_end(text, 5), 7);
        assert_eq!(line_end(text, 9), 13);
    }

    /// Character boundaries step over multi-byte characters.
    #[test]
    fn character_boundaries() {
        let text = "héllo";
        assert_eq!(next_boundary(text, 1), 3);
        assert_eq!(previous_boundary(text, 3), 1);
    }
}
