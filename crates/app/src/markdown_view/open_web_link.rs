use gpui::App;

/// URL prefixes a click may open; anything else (`file:`, app schemes,
/// relative paths) stays inert, since Markdown here is written by other people.
const OPENABLE_URL_PREFIXES: [&str; 2] = ["https://", "http://"];

/// Opens `url` in the browser when it is an absolute web URL; does nothing otherwise.
pub fn open_web_link(url: &str, cx: &App) {
    if is_openable(url) {
        cx.open_url(url);
    }
}

/// Whether a link may be opened: absolute web URLs only, in any letter case.
fn is_openable(url: &str) -> bool {
    OPENABLE_URL_PREFIXES.iter().any(|prefix| {
        url.get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Web links open, in any letter case.
    #[test]
    fn web_links_are_openable() {
        assert!(is_openable("https://github.com/a/b"));
        assert!(is_openable("HTTP://example.com"));
    }

    /// Local files, other schemes and relative paths do not open.
    #[test]
    fn other_links_are_inert() {
        assert!(!is_openable("file:///etc/passwd"));
        assert!(!is_openable("vscode://open?x"));
        assert!(!is_openable("docs/readme.md"));
    }
}
