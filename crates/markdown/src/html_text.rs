/// Entities decoded in text left over from raw HTML; pulldown-cmark already
/// decodes them in Markdown text.
const ENTITIES: [(&str, &str); 6] = [
    ("&nbsp;", " "),
    ("&lt;", "<"),
    ("&gt;", ">"),
    ("&quot;", "\""),
    ("&#39;", "'"),
    ("&amp;", "&"),
];

/// The readable text of a piece of raw HTML: comments and tags removed,
/// `<br>` turned into a line break, common entities decoded, blank lines and
/// surrounding whitespace dropped.
///
/// HTML is not rendered: PR templates use it mostly for comments and
/// `<details>` wrappers, whose text is what matters. An unterminated comment
/// or tag drops the rest of the input.
pub(crate) fn strip_html(html: &str) -> String {
    let mut text = String::new();
    let mut rest = html;

    // Walk the input, copying text and skipping each comment or tag.
    while let Some(open) = rest.find('<') {
        text.push_str(&rest[..open]);
        let markup = &rest[open..];
        let (markup_end, replacement) = match markup.strip_prefix("<!--") {
            Some(comment) => (
                comment
                    .find("-->")
                    .map(|end| "<!--".len() + end + "-->".len()),
                "",
            ),
            None => (
                markup.find('>').map(|end| end + 1),
                line_break_for_tag(markup),
            ),
        };
        let Some(markup_end) = markup_end else {
            rest = "";
            break;
        };
        text.push_str(replacement);
        rest = &markup[markup_end..];
    }
    text.push_str(rest);

    // Decode entities, then keep only non-blank, trimmed lines.
    for (entity, decoded) in ENTITIES {
        text = text.replace(entity, decoded);
    }
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// `\n` for a `<br>` tag, else nothing.
fn line_break_for_tag(tag: &str) -> &'static str {
    if is_line_break_tag(tag) { "\n" } else { "" }
}

/// Whether `tag` is a `<br>` tag, in any case, with or without `/`.
pub(crate) fn is_line_break_tag(tag: &str) -> bool {
    let name: String = tag
        .trim_start()
        .trim_start_matches('<')
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    name.eq_ignore_ascii_case("br")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Comments, including multi-line ones, leave no text.
    #[test]
    fn drops_comments() {
        assert_eq!(strip_html("<!-- describe\nyour change -->"), "");
    }

    /// Tags go, their text stays, and `<br>` breaks the line.
    #[test]
    fn keeps_text_between_tags() {
        let html = "<details>\n<summary>More</summary>\nHidden &amp; shown<br/>next\n</details>";
        assert_eq!(strip_html(html), "More\nHidden & shown\nnext");
    }

    /// An unterminated tag drops the rest instead of showing broken markup.
    #[test]
    fn unterminated_tag_drops_rest() {
        assert_eq!(strip_html("text <img src="), "text");
    }
}
