const MAX_PAGE_PATHS: usize = 256;
const MAX_PAGE_BYTES: usize = 64 * 1024;

/// A stable inventory page; `next_offset` addresses the next exact path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPage {
    pub paths: Vec<String>,
    pub total: usize,
    pub next_offset: Option<usize>,
}

impl PathPage {
    /// Limits both entry count and escaped response bytes, retaining exact paths.
    pub(crate) fn from_paths<'a>(
        paths: impl IntoIterator<Item = &'a str>,
        offset: usize,
        limit: usize,
    ) -> Self {
        let all_paths: Vec<_> = paths.into_iter().collect();
        let total = all_paths.len();
        let mut page_paths = Vec::new();
        let mut bytes = 0_usize;
        // Permit one oversized path so an exact-path page can always make progress.
        for path in all_paths
            .iter()
            .skip(offset)
            .take(limit.clamp(1, MAX_PAGE_PATHS))
        {
            let path_bytes = path.len().saturating_mul(6).saturating_add(4);
            if !page_paths.is_empty() && bytes.saturating_add(path_bytes) > MAX_PAGE_BYTES {
                break;
            }
            page_paths.push((*path).to_owned());
            bytes = bytes.saturating_add(path_bytes);
        }
        let next = offset.saturating_add(page_paths.len());
        Self {
            paths: page_paths,
            total,
            next_offset: (next < total).then_some(next),
        }
    }
}
