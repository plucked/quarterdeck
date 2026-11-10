use std::sync::Arc;

use qd_markdown::MarkdownDocument;
use qd_studio::{BriefRecord, ShownPage, Studio};

use super::diagram_geometry::DEFAULT_DIAGRAM_WIDTH;
use super::prepared_page::PreparedPage;
use super::prepared_widget::PreparedWidget;

/// The least change of the column's width that lays diagrams out again.
const WIDTH_CHANGE_THRESHOLD: f32 = 1.;

/// A written brief as the pane draws it: the studio's widgets with their
/// Markdown parsed and diagrams laid out, in one list across all pages (so
/// a widget's index names it on any page), the pages as ranges of it, and
/// the record of how it was made (for the footer and for a report; it also
/// lists the widgets the checks left out).
pub struct PreparedBrief {
    pub lead: Arc<MarkdownDocument>,
    pub widgets: Vec<PreparedWidget>,
    /// At least one page; a studio without pages gets one with no widgets.
    pub pages: Vec<PreparedPage>,
    /// The run: cost, code access, reader, prompt, raw answer, transcript,
    /// what was left out.
    pub record: BriefRecord,
    /// The width the diagrams are laid out for: the brief's column.
    diagram_width: f32,
}

impl PreparedBrief {
    /// Parses every Markdown text of `studio` and lays out its diagrams (on
    /// the background executor: a large studio has many).
    pub fn prepare(studio: Studio, record: BriefRecord) -> Self {
        // Every page's widgets into one list, each page remembering its range.
        let mut widgets = Vec::new();
        let mut pages = Vec::with_capacity(studio.pages.len().max(1));
        for page in studio.pages {
            let start = widgets.len();
            widgets.extend(page.widgets.into_iter().map(PreparedWidget::prepare));
            pages.push(PreparedPage {
                title: page.title,
                summary: page.summary,
                widgets: start..widgets.len(),
            });
        }
        if pages.is_empty() {
            pages.push(PreparedPage {
                title: String::new(),
                summary: String::new(),
                widgets: 0..0,
            });
        }
        Self {
            lead: Arc::new(MarkdownDocument::parse(&studio.lead)),
            widgets,
            pages,
            record,
            diagram_width: DEFAULT_DIAGRAM_WIDTH,
        }
    }

    /// Page `index` (from 0) as a report names it: its number from 1, the
    /// page count and its title (empty for an index past the last page).
    pub fn shown_page(&self, index: usize) -> ShownPage {
        let title = self
            .pages
            .get(index)
            .map(|page| page.title.clone())
            .unwrap_or_default();
        ShownPage {
            number: index + 1,
            count: self.pages.len(),
            title,
        }
    }

    /// Lays the diagrams out again for a column `width` wide (the reader
    /// resized the pane), so a graph turns top to bottom when the column
    /// gets too narrow for it and back when it widens. Returns the indexes
    /// of the widgets whose layout changed size: their rows need measuring
    /// again. A width within a pixel of the current one changes nothing.
    pub fn fit_diagrams(&mut self, width: f32) -> Vec<usize> {
        if (width - self.diagram_width).abs() < WIDTH_CHANGE_THRESHOLD {
            return Vec::new();
        }
        self.diagram_width = width;
        (0..self.widgets.len())
            .filter(|index| self.widgets[*index].fit_diagram(width))
            .collect()
    }
}
