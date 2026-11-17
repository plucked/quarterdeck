use serde::Deserialize;

use crate::brief_draft::{BriefIssue, BriefIssues, check_page_anchors, check_page_schema};

use super::anchor_targets::AnchorTargets;
use super::callout_tone::CalloutTone;
use super::change_scope::ChangeScope;
use super::checked_studio::CheckedStudio;
use super::code_side::CodeSide;
use super::rejected_widget::RejectedWidget;
use super::step_weight::StepWeight;
use super::studio::Studio;
use super::studio_page::StudioPage;
use super::studio_table::StudioTable;
use super::studio_tour::StudioTour;
use super::studio_widget::StudioWidget;
use super::table_role::TableRole;
use super::tour_step::TourStep;
use super::wire_anchor::WireAnchor;
use super::wire_graph::WireGraph;
use super::wire_sequence::WireSequence;

/// The most rows a table may have; longer ones are cut, so one widget
/// cannot fill the studio.
const MAX_TABLE_ROWS: usize = 40;
/// The most steps a reading order may have; a change needing more is
/// better split into several pull requests.
const MAX_TOUR_STEPS: usize = 9;
/// The most pages a studio may have; their tabs share one row.
pub(crate) const MAX_PAGES: usize = 10;

/// The agent's output as the schema describes it.
#[derive(Deserialize)]
struct WireStudio {
    lead: String,
    pages: Vec<WirePage>,
}

#[derive(Deserialize)]
struct WirePage {
    title: String,
    summary: String,
    widgets: Vec<WireWidget>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WireWidget {
    Section {
        heading: String,
        body: String,
    },
    Callout {
        tone: WireTone,
        text: String,
    },
    CodeExcerpt {
        caption: String,
        anchor: WireAnchor,
    },
    CodeCompare {
        caption: String,
        before: WireAnchor,
        after: WireAnchor,
    },
    Table(WireTable),
    Trace(WireTable),
    Scope(WireScope),
    Tour(WireTour),
    Graph(WireGraph),
    Sequence(WireSequence),
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireTone {
    Note,
    Constraint,
    Verdict,
    Evidence,
}

#[derive(Deserialize)]
struct WireTable {
    title: String,
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Deserialize)]
struct WireScope {
    changes: Vec<String>,
    unchanged: Vec<String>,
    mechanical: Vec<String>,
}

#[derive(Deserialize)]
struct WireTour {
    title: String,
    steps: Vec<WireStep>,
}

#[derive(Deserialize)]
struct WireStep {
    title: String,
    explanation: String,
    weight: WireWeight,
    anchor: Option<WireAnchor>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireWeight {
    Core,
    Supporting,
    Mechanical,
}

/// Reads an agent's structured output (shaped by
/// [`STUDIO_SCHEMA`](crate::STUDIO_SCHEMA)) into a studio of at most six
/// pages, keeping only the
/// widgets whose anchors `targets` shows and whose tables and diagrams are
/// well formed;
/// the others are listed as rejected. Output that does not match the schema
/// at all is an error.
pub fn parse_studio(
    output: serde_json::Value,
    targets: &AnchorTargets,
) -> Result<CheckedStudio, serde_json::Error> {
    let wire: WireStudio = serde_json::from_value(output)?;
    let mut pages = Vec::new();
    let mut rejected = Vec::new();
    for (index, wire_page) in wire.pages.into_iter().enumerate() {
        let title = wire_page.title.trim().to_owned();
        // Pages past the last tab are left out whole.
        if index >= MAX_PAGES {
            rejected.push(RejectedWidget {
                widget: format!("page “{title}”"),
                reason: format!("a studio has at most {MAX_PAGES} pages"),
            });
            continue;
        }
        // Check each widget on its own, so one bad anchor costs one widget.
        let mut widgets = Vec::new();
        for wire_widget in wire_page.widgets {
            let description = format!("{title}: {}", describe(&wire_widget));
            match check_widget(wire_widget, targets) {
                Ok(widget) => widgets.push(widget),
                Err(reason) => rejected.push(RejectedWidget {
                    widget: description,
                    reason,
                }),
            }
        }
        pages.push(StudioPage {
            title,
            summary: wire_page.summary.trim().to_owned(),
            widgets,
        });
    }
    Ok(CheckedStudio {
        studio: Studio {
            lead: wire.lead.trim().to_owned(),
            pages,
        },
        rejected,
    })
}

/// Builds a complete page with the same widget checks, rejecting any discarded content.
/// Unlike legacy parsing, one invalid widget rejects this entire proposed page.
pub(crate) fn check_brief_page(
    candidate: &serde_json::Value,
    targets: &AnchorTargets,
) -> Result<StudioPage, BriefIssues> {
    // Shape and anchor diagnostics retain exact pointers before typed wire decoding.
    let mut issues = BriefIssues::default();
    check_page_schema(candidate, &mut issues);
    check_page_anchors(candidate, targets, &mut issues);
    let title = candidate["title"].as_str().unwrap_or_default().trim();
    let summary = candidate["summary"].as_str().unwrap_or_default().trim();
    for (field, text) in [("title", title), ("summary", summary)] {
        if text.is_empty() {
            issues.push(BriefIssue::new(
                "empty_guidance",
                &format!("/{field}"),
                "A Brief page needs a nonempty title and summary.",
            ));
        }
    }

    // Reuse the authoritative validators, then detect the legacy path's lossy repairs.
    let mut widgets = Vec::new();
    if let Some(candidates) = candidate["widgets"].as_array() {
        for (index, candidate) in candidates.iter().enumerate() {
            let location = format!("/widgets/{index}");
            let wire = match serde_json::from_value::<WireWidget>(candidate.clone()) {
                Ok(wire) => wire,
                Err(error) => {
                    issues.push(BriefIssue::new(
                        "invalid_widget",
                        &location,
                        &error.to_string(),
                    ));
                    continue;
                }
            };
            match check_widget(wire, targets) {
                Ok(widget) => {
                    check_widget_retention(candidate, &widget, &location, &mut issues);
                    widgets.push(widget);
                }
                Err(reason) => {
                    if !issues.issues.iter().any(|issue| {
                        issue.widget_index == Some(index) && issue.code == "invalid_anchor"
                    }) {
                        issues.push(BriefIssue::new("invalid_widget", &location, &reason));
                    }
                }
            }
        }
    }
    if issues.is_empty() {
        Ok(StudioPage {
            title: title.into(),
            summary: summary.into(),
            widgets,
        })
    } else {
        Err(issues)
    }
}

/// Strict authoring rejects content the tolerant legacy parser deliberately removes.
fn check_widget_retention(
    candidate: &serde_json::Value,
    widget: &StudioWidget,
    location: &str,
    issues: &mut BriefIssues,
) {
    match widget {
        StudioWidget::Table(table) => {
            if candidate["rows"]
                .as_array()
                .is_some_and(|rows| rows.len() != table.rows.len())
            {
                issues.push(BriefIssue::new("truncated_table", &format!("{location}/rows"), &format!("A table or trace has at most {MAX_TABLE_ROWS} rows; split this content into complete widgets.")));
            }
        }
        StudioWidget::Scope(_) => {
            for field in ["changes", "unchanged", "mechanical"] {
                if let Some(items) = candidate[field].as_array() {
                    for (index, _) in items.iter().enumerate().filter(|(_, item)| {
                        item.as_str().is_some_and(|text| text.trim().is_empty())
                    }) {
                        issues.push(BriefIssue::new("discarded_scope_item", &format!("{location}/{field}/{index}"), "An empty scope entry would be discarded. Remove it explicitly or provide its explanation."));
                    }
                }
            }
        }
        _ => {}
    }
}

/// The widget's kind and title, for the rejection list.
fn describe(widget: &WireWidget) -> String {
    match widget {
        WireWidget::Section { heading, .. } => format!("section “{heading}”"),
        WireWidget::Callout { .. } => "callout".to_owned(),
        WireWidget::CodeExcerpt { caption, .. } => format!("code excerpt “{caption}”"),
        WireWidget::CodeCompare { caption, .. } => format!("code comparison “{caption}”"),
        WireWidget::Table(table) => format!("table “{}”", table.title),
        WireWidget::Trace(table) => format!("trace “{}”", table.title),
        WireWidget::Scope(_) => "scope".to_owned(),
        WireWidget::Tour(tour) => format!("reading order “{}”", tour.title),
        WireWidget::Graph(graph) => format!("graph “{}”", graph.title),
        WireWidget::Sequence(sequence) => format!("sequence “{}”", sequence.title),
    }
}

/// The widget, with its anchors checked against the diff and its table
/// shape checked; the reason otherwise.
fn check_widget(widget: WireWidget, targets: &AnchorTargets) -> Result<StudioWidget, String> {
    match widget {
        WireWidget::Section { heading, body } => Ok(StudioWidget::Section { heading, body }),
        WireWidget::Callout { tone, text } => Ok(StudioWidget::Callout {
            tone: tone.into(),
            text,
        }),
        WireWidget::CodeExcerpt { caption, anchor } => Ok(StudioWidget::CodeExcerpt {
            caption,
            anchor: anchor.checked(targets)?,
        }),
        WireWidget::CodeCompare {
            caption,
            before,
            after,
        } => {
            let before = before.checked(targets)?;
            let after = after.checked(targets)?;
            // A comparison reads from the old version on the left to the new on the right.
            if before.side != CodeSide::Base || after.side != CodeSide::Head {
                return Err("a comparison needs its before on base and its after on head".into());
            }
            Ok(StudioWidget::CodeCompare {
                caption,
                before,
                after,
            })
        }
        WireWidget::Table(table) => Ok(StudioWidget::Table(checked_table(
            table,
            TableRole::Mapping,
        )?)),
        WireWidget::Trace(table) => {
            Ok(StudioWidget::Table(checked_table(table, TableRole::Trace)?))
        }
        WireWidget::Scope(scope) => checked_scope(scope),
        WireWidget::Tour(tour) => checked_tour(tour, targets),
        WireWidget::Graph(graph) => Ok(StudioWidget::Graph(graph.checked(targets)?)),
        WireWidget::Sequence(sequence) => Ok(StudioWidget::Sequence(sequence.checked(targets)?)),
    }
}

/// The scope with blank items dropped, if anything is left.
fn checked_scope(scope: WireScope) -> Result<StudioWidget, String> {
    let keep = |items: Vec<String>| -> Vec<String> {
        items
            .into_iter()
            .map(|item| item.trim().to_owned())
            .filter(|item| !item.is_empty())
            .collect()
    };
    let scope = ChangeScope {
        changes: keep(scope.changes),
        unchanged: keep(scope.unchanged),
        mechanical: keep(scope.mechanical),
    };
    if scope.changes.is_empty() && scope.unchanged.is_empty() && scope.mechanical.is_empty() {
        return Err("a scope needs at least one item".into());
    }
    Ok(StudioWidget::Scope(scope))
}

/// The reading order, if it has 2 to [`MAX_TOUR_STEPS`] steps and every
/// step's anchor starts on a line the diff shows (anchors are cut to that
/// first line: a step links to where its piece starts); the reason naming
/// the first bad step.
fn checked_tour(tour: WireTour, targets: &AnchorTargets) -> Result<StudioWidget, String> {
    if !(2..=MAX_TOUR_STEPS).contains(&tour.steps.len()) {
        return Err(format!(
            "a reading order needs 2 to {MAX_TOUR_STEPS} steps, not {}",
            tour.steps.len()
        ));
    }
    let mut steps = Vec::with_capacity(tour.steps.len());
    for (number, step) in (1..).zip(tour.steps) {
        // A step links to where its piece starts, so only that line counts.
        let anchor = step
            .anchor
            .map(|anchor| anchor.checked_start(targets))
            .transpose()
            .map_err(|reason| format!("step {number}: {reason}"))?;
        steps.push(TourStep {
            title: step.title.trim().to_owned(),
            explanation: step.explanation.trim().to_owned(),
            weight: match step.weight {
                WireWeight::Core => StepWeight::Core,
                WireWeight::Supporting => StepWeight::Supporting,
                WireWeight::Mechanical => StepWeight::Mechanical,
            },
            anchor,
        });
    }
    Ok(StudioWidget::Tour(StudioTour {
        title: tour.title,
        steps,
    }))
}

/// The table, if it has columns and every row has one cell per column; at
/// most [`MAX_TABLE_ROWS`] rows are kept.
fn checked_table(table: WireTable, role: TableRole) -> Result<StudioTable, String> {
    if table.columns.is_empty() {
        return Err("a table needs columns".into());
    }
    if let Some(row) = table
        .rows
        .iter()
        .find(|row| row.len() != table.columns.len())
    {
        return Err(format!(
            "a row has {} cells for {} columns",
            row.len(),
            table.columns.len()
        ));
    }
    Ok(StudioTable {
        title: table.title,
        columns: table.columns,
        rows: table.rows.into_iter().take(MAX_TABLE_ROWS).collect(),
        role,
    })
}

impl From<WireTone> for CalloutTone {
    fn from(tone: WireTone) -> Self {
        match tone {
            WireTone::Note => Self::Note,
            WireTone::Constraint => Self::Constraint,
            WireTone::Verdict => Self::Verdict,
            WireTone::Evidence => Self::Evidence,
        }
    }
}
