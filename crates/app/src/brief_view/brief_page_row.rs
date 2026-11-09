use super::prepared_brief::PreparedBrief;

/// One row of a written brief's shown page, in the order the Brief pane
/// and the gallery lay them out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BriefPageRow {
    /// The lead, above the first page.
    Lead,
    /// The page's position, title and summary (briefs of several pages).
    PageIntro,
    /// Widget `index` of the brief.
    Widget(usize),
    /// Buttons to the previous and next page (briefs of several pages).
    PageNavigation,
    /// The widgets left out because their anchors, tables or diagrams did not check.
    Rejected,
    /// Who wrote the brief and what it cost.
    Footer,
}

impl BriefPageRow {
    /// The rows of page `page` of `brief` (the last page when past it):
    /// the lead on the first page; for a brief of several pages the page's
    /// heading and, at its end, the buttons to its neighbours; what was left
    /// out and the footer on the last page.
    pub fn rows_of(brief: &PreparedBrief, page: usize) -> Vec<Self> {
        let last = brief.pages.len().saturating_sub(1);
        let page = page.min(last);
        let has_pages = brief.pages.len() > 1;
        let mut rows = Vec::new();
        if page == 0 {
            rows.push(Self::Lead);
        }
        if has_pages {
            rows.push(Self::PageIntro);
        }
        if let Some(shown) = brief.pages.get(page) {
            rows.extend(shown.widgets.clone().map(Self::Widget));
        }
        if has_pages {
            rows.push(Self::PageNavigation);
        }
        if page == last {
            if !brief.record.rejected.is_empty() {
                rows.push(Self::Rejected);
            }
            rows.push(Self::Footer);
        }
        rows
    }
}
