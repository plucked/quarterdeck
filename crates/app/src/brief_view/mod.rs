//! A written brief drawn natively, shared by the pull request detail (the
//! Brief pane) and the brief gallery (every widget on sample data). The
//! view showing a brief supplies the diffs its code is quoted from and what
//! clicks do; everything else about how a brief looks lives here.
//!
//! Files:
//! - `brief_state.rs`: `BriefState`, a brief being written, written or failed.
//! - `prepared_brief.rs`: `PreparedBrief`, a written brief with its Markdown parsed.
//! - `prepared_page.rs`: `PreparedPage`, a page's title, summary and range of widgets.
//! - `brief_page_row.rs`: `BriefPageRow`, one row of a written brief's shown page, and which rows a page has.
//! - `brief_page_rendering.rs`: `render_brief_page_row`, one such row.
//! - `page_rendering.rs`: the page tabs, a page's heading, and the previous / next buttons.
//! - `prepared_step.rs`: `PreparedStep`, a reading-order step ready to draw.
//! - `tour_rendering.rs`: the reading order: numbered steps with their weight and a link to their lines.
//! - `scope_rendering.rs`: the scope: what changes, what stays the same, what is mechanical.
//! - `prepared_widget.rs`: `PreparedWidget`, one studio widget ready to draw.
//! - `brief_diffs.rs`: `BriefDiffs`, finds the diff a quoted anchor's lines come from.
//! - `brief_excerpt.rs`: `ExcerptLine` and `brief_excerpt`, the lines an anchor quotes, cut from its diff.
//! - `brief_handlers.rs`: `BriefHandlers`, what the brief's links and buttons do.
//! - `brief_widget_context.rs`: `BriefWidgetContext`, what drawing widgets needs from the view.
//! - `diagram_geometry.rs`: `DiagramPoint`, `DiagramRect`, `CubicCurve`: where diagram parts go; the default width and margin of every diagram.
//! - `text_estimate.rs`: `TextMetrics`, text sizes estimated for laying diagrams out before drawing.
//! - `graph_layout.rs`: `GraphLayout` and `lay_out_graph`: columns along the arrows, a band per boundary, routed arrows, labels kept apart.
//! - `sequence_layout.rs`: `SequenceLayout` and `lay_out_sequence`: lanes, and messages top to bottom.
//! - `dash_pattern.rs`: `DashPattern`, the dash and gap of a dashed or dotted diagram line.
//! - `diagram_stroke.rs`: `DiagramStroke` and `paint_strokes`: lines, dashes and arrowheads painted on a canvas.
//! - `graph_rendering.rs`: a graph: boundaries, arrows, boxes and labels, drawn twice (before and after) when changed.
//! - `sequence_rendering.rs`: a sequence: lane headers, lifelines, and each message's arrow, label and note.
//! - `diagram_legend.rs`: the legend under a diagram: role swatches, line samples, change marks.
//! - `diagram_rendering.rs`: what graphs and sequences share: placed parts, the stroke canvas, role colors.
//! - `familiarity_picker.rs`: how well do you know this area: the three answers and what the picked one does.
//! - `report_state.rs`: `ReportState`, where reporting the shown brief stands.
//! - `brief_report.rs`: `BriefReport` and `ReportHost`: the report's state and save task, and the steps every view shares.
//! - `report_rendering.rs`: the report panel under the version line: text field, saved path.
//! - `version_rendering.rs`: the line above a brief: the commit it was written for, against the shown head, and the Report button.
//! - `column_width_probe.rs`: `render_column_width_probe`, measures the brief's column so diagrams are laid out for it.
//! - `brief_rendering.rs`: the column and the bar over it, each widget by kind, the lead and callouts.
//! - `brief_status_rendering.rs`: the status card: no brief yet, one being written (how far), or a failed run.
//! - `brief_text_rendering.rs`: the brief's Markdown prose and the caption above code.
//! - `code_excerpt_rendering.rs`: quoted lines under their anchor link, in the diff's look, with gap rows.
//! - `table_rendering.rs`: tables of Markdown cells; a trace numbers its rows.
//! - `brief_footer_rendering.rs`: the footer (what the brief was written from, Write again) and the left-out list.

mod brief_diffs;
mod brief_excerpt;
mod brief_footer_rendering;
mod brief_handlers;
mod brief_page_rendering;
mod brief_page_row;
mod brief_rendering;
mod brief_report;
mod brief_state;
mod brief_status_rendering;
mod brief_text_rendering;
mod brief_widget_context;
mod code_excerpt_rendering;
mod column_width_probe;
mod dash_pattern;
mod diagram_geometry;
mod diagram_legend;
mod diagram_rendering;
mod diagram_stroke;
mod familiarity_picker;
mod graph_layout;
mod graph_rendering;
mod page_rendering;
mod prepared_brief;
mod prepared_page;
mod prepared_step;
mod prepared_widget;
mod report_rendering;
mod report_state;
mod scope_rendering;
mod sequence_layout;
mod sequence_rendering;
mod table_rendering;
mod text_estimate;
mod tour_rendering;
mod version_rendering;

pub use brief_diffs::BriefDiffs;
pub use brief_handlers::{BriefHandlers, view_action, view_handler};
pub use brief_page_rendering::render_brief_page_row;
pub use brief_page_row::BriefPageRow;
pub use brief_rendering::{render_brief_bar, render_brief_column};
pub use brief_report::{BriefReport, ReportHost};
pub use brief_state::BriefState;
pub use brief_status_rendering::render_brief_status;
pub use brief_widget_context::BriefWidgetContext;
pub use column_width_probe::render_column_width_probe;
pub use diagram_geometry::DEFAULT_DIAGRAM_WIDTH;
pub use page_rendering::render_page_tabs;
pub use prepared_brief::PreparedBrief;
pub use prepared_widget::PreparedWidget;
pub use report_rendering::render_report_panel;
pub use report_state::ReportState;
pub use version_rendering::render_brief_version;
