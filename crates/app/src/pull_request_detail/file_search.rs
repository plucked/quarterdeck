use std::collections::HashMap;

use super::content_match::ContentMatch;
use super::file_search_match::FileSearchMatch;
use super::search_scope::SearchScope;

/// Enough preceding characters to orient a match while keeping it visible in a narrow drawer.
const SNIPPET_CONTEXT_CHARS: usize = 8;
const MAX_SNIPPET_CHARS: usize = 180;
const MAX_MATCH_LINES: usize = 3;

/// The query and its derived matches; empty queries admit all files.
#[derive(Default)]
pub(super) struct FileSearch {
    pub query: String,
    pub scope: SearchScope,
    pub matches: HashMap<String, FileSearchMatch>,
}

impl FileSearch {
    pub fn admits(&self, path: &str) -> bool {
        self.query.trim().is_empty() || self.matches.contains_key(path)
    }
}

/// Matches tokens as case-insensitive subsequences, favoring contiguous filename matches.
pub(super) fn fuzzy_path(path: &str, query: &str) -> Option<FileSearchMatch> {
    let path = path.to_lowercase();
    let filename = path.rsplit('/').next().unwrap_or(&path);
    let mut score = 0;
    for token in query.split_whitespace().map(str::to_lowercase) {
        if !contains_subsequence(&path, &token) {
            return None;
        }
        if contains_subsequence(filename, &token) {
            score += 50;
        }
        score += 1;
        if path.contains(&token) {
            score += 20;
        }
        if filename.contains(&token) {
            score += 80;
        }
        if filename.starts_with(&token) {
            score += 40;
        }
    }
    Some(FileSearchMatch {
        score,
        ..FileSearchMatch::default()
    })
}

/// Tests a token without requiring adjacent letters.
fn contains_subsequence(text: &str, token: &str) -> bool {
    let mut remaining = text.chars();
    token
        .chars()
        .all(|character| remaining.any(|candidate| candidate == character))
}

/// Searches all lines, including unchanged text; only three snippets are retained.
pub(super) fn content_matches(text: &str, query: &str) -> Option<FileSearchMatch> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return None;
    }
    let mut result = FileSearchMatch::default();
    for (index, line) in text.lines().enumerate() {
        if line.to_lowercase().contains(&query) {
            result.line_count += 1;
            if result.lines.len() < MAX_MATCH_LINES {
                result.lines.push(matching_excerpt(
                    line,
                    &query,
                    u32::try_from(index + 1).unwrap_or(u32::MAX),
                ));
            }
        }
    }
    (result.line_count > 0).then_some(result)
}

/// Builds a UTF-8-safe excerpt around the first match, including case mappings that expand a character.
fn matching_excerpt(line: &str, query: &str, line_number: u32) -> ContentMatch {
    // Map the lowercase match back to original bytes; lowercase can change a character's byte length.
    let normalized_start = line.to_lowercase().find(query).unwrap_or(0);
    let normalized_end = normalized_start + query.len();
    let mut normalized_offset = 0;
    let mut source_start = None;
    let mut source_end = 0;
    for (byte, character) in line.char_indices() {
        let end = normalized_offset + character.to_lowercase().map(char::len_utf8).sum::<usize>();
        if end > normalized_start && normalized_offset < normalized_end {
            source_start.get_or_insert(byte);
            source_end = byte + character.len_utf8();
        }
        normalized_offset = end;
        if end >= normalized_end {
            break;
        }
    }
    let source_start = source_start.unwrap_or(0);

    // Keep only a small prefix before the match, rather than spending the viewport on column one.
    let preceding = line[..source_start]
        .chars()
        .count()
        .saturating_sub(SNIPPET_CONTEXT_CHARS);
    let context_start = line
        .char_indices()
        .nth(preceding)
        .map_or(0, |(byte, _)| byte);
    let mut text: String = line[context_start..]
        .chars()
        .take(MAX_SNIPPET_CHARS)
        .collect();
    let mut prefix_length = 0;
    if context_start > 0 {
        text.insert(0, '…');
        prefix_length = '…'.len_utf8();
    }
    let matched = source_start - context_start + prefix_length
        ..(source_end - context_start + prefix_length).min(text.len());
    ContentMatch {
        line: line_number,
        text,
        matched,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fuzzy filename and directory tokens accept omissions without accepting missing letters.
    #[test]
    fn paths_are_fuzzy_and_rank_filenames_first() {
        assert!(fuzzy_path("engine/runtime/SectorState.rs", "eng sctrst").is_some());
        assert!(fuzzy_path("engine/runtime/SectorState.rs", "zz").is_none());
        assert!(
            fuzzy_path("src/sector.rs", "sector").unwrap().score
                > fuzzy_path("sector/src/lib.rs", "sector").unwrap().score
        );
    }

    /// Content search reaches beyond a diff preview, matches Unicode and bounds only snippets.
    #[test]
    fn complete_contents_include_unchanged_lines() {
        let text = format!(
            "{}\nÅngström\nångström\nÅNGSTRÖM\nångström",
            "unchanged\n".repeat(400)
        );
        let found = content_matches(&text, "ÅNGSTRÖM").unwrap();
        assert_eq!(found.line_count, 4);
        assert_eq!(found.lines.len(), 3);
        assert_eq!(found.lines[0].line, 402);
    }
    /// Late-column hits remain visible and highlight byte ranges survive expanding Unicode case mappings.
    #[test]
    fn excerpts_begin_near_the_match_and_keep_utf8_boundaries() {
        let line = format!(
            "{} string RequiredVersion, bool enabled",
            "parameter, ".repeat(40)
        );
        let found = content_matches(&line, "requiredversion").unwrap();
        let hit = &found.lines[0];
        assert!(hit.text.starts_with('…'));
        assert_eq!(&hit.text[hit.matched.clone()], "RequiredVersion");
        assert!(hit.text[..hit.matched.start].chars().count() <= SNIPPET_CONTEXT_CHARS + 1);
        let unicode = content_matches(&format!("{} İSTANBUL", "π".repeat(200)), "i").unwrap();
        assert_eq!(
            &unicode.lines[0].text[unicode.lines[0].matched.clone()],
            "İ"
        );
    }
}
