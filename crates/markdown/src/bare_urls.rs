use std::ops::Range;

/// Schemes a bare URL may start with, as GitHub autolinks them.
const URL_SCHEMES: [&str; 2] = ["https://", "http://"];
/// Characters that end a sentence rather than the URL when they come last.
const TRAILING_PUNCTUATION: [char; 9] = ['.', ',', ':', ';', '!', '?', '\'', '"', ')'];

/// Byte ranges of the bare web URLs in `text` (GitHub's autolink literals):
/// a URL runs from its scheme to the next whitespace or `<`, minus trailing
/// sentence punctuation. A `)` stays when the URL opened one, as in
/// Wikipedia links.
pub(crate) fn find_bare_urls(text: &str) -> Vec<Range<usize>> {
    let mut urls = Vec::new();
    let mut search_from = 0;

    // Find the earliest scheme from where the last URL ended.
    while let Some(start) = URL_SCHEMES
        .iter()
        .filter_map(|scheme| text[search_from..].find(scheme))
        .min()
        .map(|offset| search_from + offset)
    {
        let length = text[start..]
            .find(|character: char| character.is_whitespace() || character == '<')
            .unwrap_or(text.len() - start);
        let url = trim_trailing_punctuation(&text[start..start + length]);
        let is_scheme_only = URL_SCHEMES.iter().any(|scheme| url.len() <= scheme.len());
        if !is_scheme_only {
            urls.push(start..start + url.len());
        }
        search_from = start + length.max(1);
    }
    urls
}

/// Drops trailing punctuation, keeping a `)` that closes a `(` inside the URL.
fn trim_trailing_punctuation(url: &str) -> &str {
    let mut trimmed = url;
    while let Some(last) = trimmed.chars().last() {
        let is_balanced_paren =
            last == ')' && trimmed.matches('(').count() >= trimmed.matches(')').count();
        if !TRAILING_PUNCTUATION.contains(&last) || is_balanced_paren {
            break;
        }
        trimmed = &trimmed[..trimmed.len() - last.len_utf8()];
    }
    trimmed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn urls(text: &str) -> Vec<&str> {
        find_bare_urls(text)
            .into_iter()
            .map(|range| &text[range])
            .collect()
    }

    /// URLs end at whitespace and lose sentence punctuation.
    #[test]
    fn finds_urls_without_trailing_punctuation() {
        assert_eq!(
            urls("See https://a.dev/x, and http://b.dev/y."),
            ["https://a.dev/x", "http://b.dev/y"]
        );
    }

    /// A closing parenthesis stays when the URL opened one.
    #[test]
    fn keeps_balanced_parenthesis() {
        assert_eq!(
            urls("(https://en.wikipedia.org/wiki/A_(b))"),
            ["https://en.wikipedia.org/wiki/A_(b)"]
        );
    }

    /// A scheme with nothing after it is not a link.
    #[test]
    fn ignores_bare_scheme() {
        assert_eq!(urls("type https:// then"), Vec::<&str>::new());
    }
}
