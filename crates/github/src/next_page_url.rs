/// The `rel="next"` URL of a `Link` header, as GitHub paginates lists:
/// `<https://api.github.com/…&page=2>; rel="next", <…>; rel="last"`.
pub(crate) fn next_page_url(link_header: &str) -> Option<String> {
    link_header.split(',').find_map(|link| {
        let (url, parameters) = link.split_once(';')?;
        let is_next = parameters
            .split(';')
            .any(|parameter| parameter.trim() == r#"rel="next""#);
        is_next.then(|| {
            url.trim()
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_owned()
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The next page is found among the other relations.
    #[test]
    fn finds_next_page() {
        let header = r#"<https://api.github.com/repositories/1/pulls/2/files?per_page=100&page=2>; rel="next", <https://api.github.com/repositories/1/pulls/2/files?per_page=100&page=3>; rel="last""#;
        assert_eq!(
            next_page_url(header).as_deref(),
            Some("https://api.github.com/repositories/1/pulls/2/files?per_page=100&page=2")
        );
    }

    /// The last page has no next relation.
    #[test]
    fn last_page_has_no_next() {
        let header = r#"<https://api.github.com/x?page=1>; rel="prev", <https://api.github.com/x?page=1>; rel="first""#;
        assert_eq!(next_page_url(header), None);
    }
}
