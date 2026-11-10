use gpui::{
    AnyElement, Context, FontWeight, IntoElement, Render, Rgba, SharedString, Window, div, list,
    prelude::*, px, relative,
};
use qd_github::CheckOutcome;

use super::check_notes::{access_note, conflict_note, footnote};
use super::check_outcome_mark::CheckOutcomeMark;
use super::check_row::{CheckRow, CheckSection, SkippedChecks};
use super::check_row_rendering::{
    NodeRow, TREE_INDENT, render_flagged_row, render_job_row, render_node_row,
};
use super::check_tally::CheckTally;
use super::check_tree::{CheckTree, TreeEntry};
use super::checks_panel::ChecksPanel;
use super::checks_summary::Summary;
use super::shown_checks::ShownChecks;
use crate::button::Button;
use crate::count_as_f32::count_as_f32;
use crate::fold_chevron::FoldState;
use crate::pulsing_status::PulsingStatus;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::section_label::SectionLabel;
use crate::theme;

/// Diameter of the mark beside the verdict.
const VERDICT_MARK_SIZE: f32 = 18.;
/// Height of the outcome bar under the verdict.
const OUTCOME_BAR_HEIGHT: f32 = 4.;
/// Gap between the outcome bar's segments.
const OUTCOME_BAR_GAP: f32 = 1.;
/// How far the verdict's mark sits down, to center it on the verdict's first line.
const VERDICT_MARK_DROP: f32 = 1.;
/// Height of a section heading's line, so headings with and without the
/// skipped switch take the same room.
const HEADING_ROW_HEIGHT: f32 = 22.;

impl Render for ChecksPanel {
    /// The verdict over the checks, then the checks: what failed and what
    /// still runs, flat, then every check by workflow, folded.
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel = div().flex_1().min_h_0().flex().flex_col();
        if let Some(reason) = self.connection_error() {
            return panel.child(render_note(
                format!("Checks need a GitHub connection: {reason}"),
                theme::muted(),
            ));
        }
        let Some(shown) = self.shown() else {
            let note = match (self.error(), self.is_loading()) {
                (Some(error), _) => render_note(
                    format!("The checks could not be read: {error}"),
                    theme::danger(),
                ),
                (None, true) => div()
                    .p_3()
                    .child(PulsingStatus::new(
                        "checks-first-read",
                        "Reading the checks from GitHub…",
                    ))
                    .into_any_element(),
                (None, false) => render_note("No checks read yet.", theme::muted()),
            };
            return panel.child(note);
        };
        panel
            .child(self.render_summary(shown, cx))
            .children(self.render_check_list(shown, cx))
    }
}

impl ChecksPanel {
    /// The verdict with its mark, the counts, the outcome bar, the required
    /// checks, the commit and when it was read, and Refresh.
    fn render_summary(&self, shown: &ShownChecks, cx: &mut Context<Self>) -> impl IntoElement {
        let summary = match shown.content.tree() {
            Some(tree) => Summary::of_tree(tree),
            None => Summary::of_rollup(shown),
        }
        .with_conflict(shown.conflict.as_ref());
        div()
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .pt_2()
            .pb_3()
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(div().pt(px(VERDICT_MARK_DROP)).child(CheckOutcomeMark::new(
                        "checks-verdict",
                        summary.outcome.unwrap_or(CheckOutcome::Neutral),
                        VERDICT_MARK_SIZE,
                    )))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme::text())
                                    .child(summary.verdict),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme::muted())
                                    .child(summary.counts),
                            ),
                    )
                    .child(self.render_refresh_control(cx)),
            )
            .children(summary.bar.map(render_outcome_bar))
            .child(
                div()
                    .text_xs()
                    .text_color(theme::subtle())
                    .child(footnote(shown, summary.required.as_ref())),
            )
            .children(self.error().map(|error| {
                render_inline_note(format!("Refresh failed: {error}"), theme::danger())
            }))
            .children(conflict_note(shown).map(|note| render_inline_note(note, theme::danger())))
            .children(access_note(shown).map(|note| render_inline_note(note, theme::muted())))
    }

    /// Refresh, or while the checks are read, a pulsing status in its place
    /// (as the top bar pulses while the queue refreshes).
    fn render_refresh_control(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.is_loading() {
            return div()
                .flex_none()
                .pt_0p5()
                .child(PulsingStatus::new("checks-reading", "Reading…"))
                .into_any_element();
        }
        Button::new("checks-refresh", "Refresh")
            .ghost()
            .on_click(cx.listener(|panel, _, _, cx| panel.load(cx)))
            .into_any_element()
    }

    /// The rows under the summary in a list with its scrollbar; nothing
    /// when there are no single checks to show.
    fn render_check_list(&self, shown: &ShownChecks, cx: &mut Context<Self>) -> Option<AnyElement> {
        let tree = shown.content.tree()?;
        if tree.tally().total() == 0 {
            return None;
        }
        Some(
            div()
                .relative()
                .flex_1()
                .min_h_0()
                .child(
                    list(
                        self.list().clone(),
                        cx.processor(|panel, index, _window, cx| panel.render_check_row(index, cx)),
                    )
                    .size_full()
                    .pb_3(),
                )
                .child(VerticalScrollbar::new(
                    "checks-scrollbar",
                    ScrollbarTarget::List(self.list().clone()),
                ))
                .into_any_element(),
        )
    }

    /// Row `index` of the list.
    fn render_check_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(tree) = self.shown().and_then(|shown| shown.content.tree()) else {
            return div().into_any_element();
        };
        let Some(row) = self.rows().get(index).copied() else {
            return div().into_any_element();
        };
        match row {
            CheckRow::Heading(section) => self.render_heading(section, tree, cx),
            CheckRow::Flagged(check) => render_flagged_row(tree, check, cx),
            CheckRow::Workflow(workflow) => match FoldableNode::workflow(tree, workflow) {
                Some(node) => self.render_foldable_row(index, node, cx),
                None => div().into_any_element(),
            },
            CheckRow::Group { workflow, entry } => match FoldableNode::group(tree, workflow, entry)
            {
                Some(node) => self.render_foldable_row(index, node, cx),
                None => div().into_any_element(),
            },
            CheckRow::Job { check, depth } => render_job_row(tree, check, depth, cx),
        }
    }

    /// A section's small uppercase label with its count; the tree's has
    /// the switch showing or hiding what was skipped.
    fn render_heading(
        &self,
        section: CheckSection,
        tree: &CheckTree,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title = match section {
            CheckSection::Failed => format!("Failed · {}", tree.broken().len()),
            CheckSection::InProgress => format!("In progress · {}", tree.pending().len()),
            CheckSection::Expected => format!("Expected · {}", tree.expected().len()),
            CheckSection::All => format!("All checks · {}", tree.tally().total()),
        };
        let skipped_count = tree.tally().count(CheckOutcome::Skipped);
        let skipped_switch = (section == CheckSection::All && skipped_count > 0).then(|| {
            let label = match self.skipped() {
                SkippedChecks::Hidden => format!("Show {skipped_count} skipped"),
                SkippedChecks::Shown => "Hide skipped".to_owned(),
            };
            Button::new("checks-skipped-switch", label)
                .ghost()
                .on_click(cx.listener(|panel, _, _, cx| panel.toggle_skipped(cx)))
        });
        // The switch is a ghost button: its padding keeps it apart from the
        // label, and the row's right padding is trimmed by as much, so its
        // text lines up with the rows' right edge.
        div()
            .pl_3()
            .pr_1()
            .pt_3()
            .pb_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .h(px(HEADING_ROW_HEIGHT))
                    .child(SectionLabel::new(title).compact().truncate())
                    .children(skipped_switch),
            )
            .into_any_element()
    }

    /// A workflow or a group in one: chevron, its outcome as one, name, a
    /// pill for the required checks inside, and its progress; a group is
    /// indented and lighter. A click folds or unfolds it.
    fn render_foldable_row(
        &self,
        row_index: usize,
        node: FoldableNode,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let workflow = node.workflow.to_owned();
        let group = node.group.map(str::to_owned);
        let is_unfolded = self.unfolded().contains(&workflow, group.as_deref());
        render_node_row(NodeRow {
            id: (node.id_prefix(), row_index).into(),
            indent: node.indent(),
            name: node.name().to_owned().into(),
            tally: node.tally,
            required: node.required,
            fold: FoldState::of(!is_unfolded),
            weight: node.weight(),
        })
        .on_click(cx.listener(move |panel, _, _, cx| {
            panel.toggle_fold(&workflow, group.as_deref(), cx);
        }))
        .into_any_element()
    }
}

/// A workflow, or a group in one: what its foldable row shows and folds.
#[derive(Clone, Copy)]
struct FoldableNode<'a> {
    workflow: &'a str,
    /// The group's name; `None` for the workflow itself.
    group: Option<&'a str>,
    tally: CheckTally,
    /// The required checks inside, which the row's pill counts.
    required: CheckTally,
}

impl<'a> FoldableNode<'a> {
    /// Workflow `workflow_index` of `tree`.
    fn workflow(tree: &'a CheckTree, workflow_index: usize) -> Option<Self> {
        let workflow = tree.workflows().get(workflow_index)?;
        Some(Self {
            workflow: &workflow.name,
            group: None,
            tally: workflow.tally,
            required: workflow.required,
        })
    }

    /// Entry `entry_index` of workflow `workflow_index`, when it is a group.
    fn group(tree: &'a CheckTree, workflow_index: usize, entry_index: usize) -> Option<Self> {
        let workflow = tree.workflows().get(workflow_index)?;
        let Some(TreeEntry::Group(group)) = workflow.entries.get(entry_index) else {
            return None;
        };
        Some(Self {
            workflow: &workflow.name,
            group: Some(&group.name),
            tally: group.tally,
            required: group.required,
        })
    }

    /// The name the row shows: the group's, else the workflow's.
    fn name(&self) -> &'a str {
        self.group.unwrap_or(self.workflow)
    }

    /// The first part of the row's element id.
    fn id_prefix(&self) -> &'static str {
        match self.group {
            None => "check-workflow",
            Some(_) => "check-group",
        }
    }

    /// How far the row is indented: a group sits one level in.
    fn indent(&self) -> f32 {
        match self.group {
            None => 0.,
            Some(_) => TREE_INDENT,
        }
    }

    /// The name's weight: workflows stand out from their groups.
    fn weight(&self) -> FontWeight {
        match self.group {
            None => FontWeight::MEDIUM,
            Some(_) => FontWeight::NORMAL,
        }
    }
}

/// The bar of outcomes that ran, each segment as wide as its share.
fn render_outcome_bar(tally: CheckTally) -> impl IntoElement {
    let ran = tally.ran();
    div()
        .flex()
        .gap(px(OUTCOME_BAR_GAP))
        .h(px(OUTCOME_BAR_HEIGHT))
        .rounded_md()
        .overflow_hidden()
        .bg(theme::border())
        .children(
            tally
                .nonzero()
                .filter(|(outcome, _)| *outcome != CheckOutcome::Skipped)
                .map(|(outcome, count)| {
                    div()
                        .h_full()
                        .w(relative(share(count, ran)))
                        .bg(bar_color(outcome))
                }),
        )
}

/// A quiet line in place of the checks.
fn render_note(text: impl Into<SharedString>, color: Rgba) -> AnyElement {
    div()
        .p_3()
        .text_xs()
        .text_color(color)
        .child(text.into())
        .into_any_element()
}

/// A line inside the summary.
fn render_inline_note(text: impl Into<SharedString>, color: Rgba) -> AnyElement {
    div()
        .text_xs()
        .text_color(color)
        .child(text.into())
        .into_any_element()
}

/// An outcome's color in the bar, matching its mark.
fn bar_color(outcome: CheckOutcome) -> Rgba {
    match outcome {
        CheckOutcome::Failed => theme::danger(),
        CheckOutcome::Running => theme::accent(),
        CheckOutcome::Queued => theme::border_strong(),
        CheckOutcome::Expected => theme::border(),
        CheckOutcome::Cancelled => theme::muted(),
        CheckOutcome::Neutral | CheckOutcome::Skipped => theme::subtle(),
        CheckOutcome::Passed => theme::success(),
    }
}

/// `part` of `whole` as a fraction (0 for nothing of nothing).
fn share(part: usize, whole: usize) -> f32 {
    match whole {
        0 => 0.,
        _ => count_as_f32(part) / count_as_f32(whole),
    }
}
