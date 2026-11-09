use gpui::{
    AnyElement, Context, FontWeight, IntoElement, Render, SharedString, Window, div, list,
    prelude::*,
};

use qd_studio::{CalloutTone, TableRole};

use super::brief_gallery::BriefGallery;
use super::gallery_row::GalleryRow;
use super::sample_brief::SampleBrief;
use crate::app_commands::CloseWindow;
use crate::brief_view::{
    BriefPageRow, BriefWidgetContext, PreparedBrief, PreparedWidget, render_brief_column,
    render_brief_page_row, render_brief_status, render_brief_version, render_column_width_probe,
    render_page_tabs, render_report_panel,
};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// What the intro row says.
const INTRO: &str = "Every part of a brief, drawn by the same code as the Brief pane, on a \
    hand-written sample built on the commit that made a used-up rate limit hold the refresh \
    schedule. The label above each part names it. The sample has several pages: the tabs and \
    the previous / next buttons switch them. Anchor links and the other buttons do nothing here. \
    Resize the window to see narrow layouts; switch the theme in Settings. Run `bacon gallery` to rebuild and reopen this window on every save.";

impl Render for BriefGallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus)
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .relative()
            .size_full()
            .bg(theme::background())
            .text_color(theme::text())
            .text_sm()
            .child(
                list(
                    self.list.clone(),
                    cx.processor(|gallery, index, _window, _cx| gallery.render_row(index)),
                )
                .size_full(),
            )
            // Measures the column, so the diagrams follow the window's width.
            .child(render_column_width_probe(self.column_width, {
                let gallery = cx.entity().downgrade();
                move |width, cx| {
                    gallery
                        .update(cx, |gallery, cx| gallery.set_column_width(width, cx))
                        .ok();
                }
            }))
            .child(VerticalScrollbar::new(
                "gallery-scrollbar",
                ScrollbarTarget::List(self.list.clone()),
            ))
    }
}

impl BriefGallery {
    /// Row `index` in the brief's column, its own element id
    /// keeping the ids of repeated parts (status buttons) apart.
    fn render_row(&self, index: usize) -> AnyElement {
        let Some(row) = self.rows.get(index).copied() else {
            return div().into_any_element();
        };
        let content = match (&self.sample, row) {
            (_, GalleryRow::Intro) => div()
                .text_sm()
                .text_color(theme::muted())
                .child(INTRO)
                .into_any_element(),
            (_, GalleryRow::Heading(heading)) => div()
                .pt_4()
                .pb_1()
                .border_b_1()
                .border_color(theme::border())
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::subtle())
                .child(heading.to_uppercase())
                .into_any_element(),
            (Err(message), GalleryRow::Broken) => div()
                .text_color(theme::error())
                .child(message.clone())
                .into_any_element(),
            (Ok(sample), row) => self.render_sample_row(sample, row),
            (Err(_), _) => div().into_any_element(),
        };
        div()
            .id(("gallery-row", index))
            .child(render_brief_column(index == 0, content))
            .into_any_element()
    }

    /// A row showing a part of `sample`, under the label naming it.
    fn render_sample_row(&self, sample: &SampleBrief, row: GalleryRow) -> AnyElement {
        let widget_context = BriefWidgetContext {
            diffs: &sample.diffs,
            reader: self.reader,
            handlers: &self.handlers,
        };
        let (label, part) = match row {
            GalleryRow::Status(index) => match sample.statuses.get(index) {
                Some(status) => (
                    format!("status · {}", status.label),
                    render_brief_status(
                        status.state.as_ref(),
                        status.can_write,
                        self.reader,
                        &self.handlers,
                    ),
                ),
                None => return div().into_any_element(),
            },
            GalleryRow::Version => (
                "version line and report panel (pinned above the brief in the Brief pane)"
                    .to_owned(),
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(render_brief_version(
                        &sample.brief.record,
                        sample.brief.record.head_sha.as_deref(),
                        self.report.state(),
                        &self.handlers,
                    ))
                    .children(render_report_panel(self.report.state(), &self.handlers))
                    .into_any_element(),
            ),
            GalleryRow::PageTabs => (
                "page tabs (pinned above the brief in the Brief pane)".to_owned(),
                render_page_tabs(&sample.brief.pages, self.page, &self.handlers),
            ),
            GalleryRow::Page(page_row) => (
                page_row_label(&sample.brief, page_row),
                render_brief_page_row(&sample.brief, self.page, page_row, &widget_context),
            ),
            GalleryRow::Intro | GalleryRow::Heading(_) | GalleryRow::Broken => {
                return div().into_any_element();
            }
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .font_family(theme::MONO_FONT)
                    .text_xs()
                    .text_color(theme::subtle())
                    .child(SharedString::from(label)),
            )
            .child(part)
            .into_any_element()
    }
}

/// The label naming a page row's part.
fn page_row_label(brief: &PreparedBrief, row: BriefPageRow) -> String {
    match row {
        BriefPageRow::Lead => "lead".to_owned(),
        BriefPageRow::PageIntro => "page heading".to_owned(),
        BriefPageRow::Widget(index) => brief
            .widgets
            .get(index)
            .map(widget_label)
            .unwrap_or_default(),
        BriefPageRow::PageNavigation => "page navigation".to_owned(),
        BriefPageRow::Rejected => "left out".to_owned(),
        BriefPageRow::Footer => "footer".to_owned(),
    }
}

/// The widget's kind as the agent writes it, with its tone or role.
fn widget_label(widget: &PreparedWidget) -> String {
    match widget {
        PreparedWidget::Section { .. } => "section".to_owned(),
        PreparedWidget::Callout { tone, .. } => match tone {
            CalloutTone::Note => "callout · note".to_owned(),
            CalloutTone::Constraint => "callout · constraint".to_owned(),
            CalloutTone::Verdict => "callout · verdict".to_owned(),
            CalloutTone::Evidence => "callout · evidence".to_owned(),
        },
        PreparedWidget::CodeExcerpt { .. } => "code_excerpt".to_owned(),
        PreparedWidget::CodeCompare { .. } => "code_compare".to_owned(),
        PreparedWidget::Table { role, .. } => match role {
            TableRole::Mapping => "table".to_owned(),
            TableRole::Trace => "trace".to_owned(),
        },
        PreparedWidget::Graph { graph, .. } if graph.has_changes() => {
            "graph · before and after".to_owned()
        }
        PreparedWidget::Graph { .. } => "graph".to_owned(),
        PreparedWidget::Sequence { .. } => "sequence".to_owned(),
        PreparedWidget::Scope { .. } => "scope".to_owned(),
        PreparedWidget::Tour { .. } => "tour (reading order)".to_owned(),
    }
}
