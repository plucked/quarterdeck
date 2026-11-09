/// The last `max_chars` characters of `text`: the end of a tool's error
/// output, where it says what went wrong.
pub(crate) fn tail(text: &str, max_chars: usize) -> String {
    let skip = text.chars().count().saturating_sub(max_chars);
    text.chars().skip(skip).collect()
}
