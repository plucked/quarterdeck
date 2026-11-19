/// Repeats the quote markers and indentation before a parsed block on its next line.
/// A list marker on the opening line becomes padding under that item.
pub fn container_continuation(prefix: &str) -> String {
    prefix
        .chars()
        .map(|character| match character {
            '>' | '\t' => character,
            _ => ' ',
        })
        .collect()
}
