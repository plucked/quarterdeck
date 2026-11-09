use std::sync::Arc;

use qd_markdown::MarkdownDocument;
use qd_studio::{BriefRecord, ShownPage, Studio, StudioWidget};

use super::graph_layout::{DEFAULT_DIAGRAM_WIDTH, lay_out_graph};
use super::prepared_page::PreparedPage;
use super::prepared_step::PreparedStep;
use super::prepared_widget::PreparedWidget;
use super::sequence_layout::lay_out_sequence;

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
        let parse = |text: &str| Arc::new(MarkdownDocument::parse(text));
        let prepare_widget = |widget| match widget {
            StudioWidget::Section { heading, body } => PreparedWidget::Section {
                heading,
                body: parse(&body),
            },
            StudioWidget::Callout { tone, text } => PreparedWidget::Callout {
                tone,
                text: parse(&text),
            },
            StudioWidget::CodeExcerpt { caption, anchor } => {
                PreparedWidget::CodeExcerpt { caption, anchor }
            }
            StudioWidget::CodeCompare {
                caption,
                before,
                after,
            } => PreparedWidget::CodeCompare {
                caption,
                before,
                after,
            },
            StudioWidget::Table(table) => PreparedWidget::Table {
                rows: table
                    .rows
                    .iter()
                    .map(|row| row.iter().map(|cell| parse(cell)).collect())
                    .collect(),
                title: table.title,
                columns: table.columns,
                role: table.role,
            },
            StudioWidget::Scope(scope) => {
                let parse_all = |items: &[String]| items.iter().map(|item| parse(item)).collect();
                PreparedWidget::Scope {
                    changes: parse_all(&scope.changes),
                    unchanged: parse_all(&scope.unchanged),
                    mechanical: parse_all(&scope.mechanical),
                }
            }
            StudioWidget::Tour(tour) => PreparedWidget::Tour {
                steps: tour
                    .steps
                    .into_iter()
                    .map(|step| PreparedStep {
                        explanation: parse(&step.explanation),
                        title: step.title,
                        weight: step.weight,
                        anchor: step.anchor,
                    })
                    .collect(),
                title: tour.title,
            },
            StudioWidget::Graph(graph) => PreparedWidget::Graph {
                layout: lay_out_graph(&graph, DEFAULT_DIAGRAM_WIDTH),
                graph,
            },
            StudioWidget::Sequence(sequence) => PreparedWidget::Sequence {
                layout: lay_out_sequence(&sequence, DEFAULT_DIAGRAM_WIDTH),
                sequence,
            },
        };

        // Every page's widgets into one list, each page remembering its range.
        let mut widgets = Vec::new();
        let mut pages = Vec::with_capacity(studio.pages.len().max(1));
        for page in studio.pages {
            let start = widgets.len();
            widgets.extend(page.widgets.into_iter().map(prepare_widget));
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
            lead: parse(&studio.lead),
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
        if (width - self.diagram_width).abs() < 1. {
            return Vec::new();
        }
        self.diagram_width = width;
        let mut changed = Vec::new();
        for (index, widget) in self.widgets.iter_mut().enumerate() {
            let resized = match widget {
                PreparedWidget::Graph { graph, layout } => {
                    let fitted = lay_out_graph(graph, width);
                    let resized =
                        is_resized((layout.width, layout.height), (fitted.width, fitted.height));
                    *layout = fitted;
                    resized
                }
                PreparedWidget::Sequence { sequence, layout } => {
                    let fitted = lay_out_sequence(sequence, width);
                    let resized =
                        is_resized((layout.width, layout.height), (fitted.width, fitted.height));
                    *layout = fitted;
                    resized
                }
                PreparedWidget::Section { .. }
                | PreparedWidget::Callout { .. }
                | PreparedWidget::CodeExcerpt { .. }
                | PreparedWidget::CodeCompare { .. }
                | PreparedWidget::Table { .. }
                | PreparedWidget::Scope { .. }
                | PreparedWidget::Tour { .. } => false,
            };
            if resized {
                changed.push(index);
            }
        }
        changed
    }
}

/// Whether a diagram's size changed by half a pixel or more.
fn is_resized(before: (f32, f32), after: (f32, f32)) -> bool {
    (before.0 - after.0).abs() >= 0.5 || (before.1 - after.1).abs() >= 0.5
}
