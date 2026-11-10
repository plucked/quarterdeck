use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, Div, FontWeight, IntoElement, Render, Rgba,
    SharedString, Stateful, Styled, Window, div, list, prelude::*, pulsating_between, px, relative,
};
use qd_github::{CheckOutcome, ConflictLocation, MergeConflict, StatusCheck};

use super::check_outcome_mark::CheckOutcomeMark;
use super::check_row::{CheckRow, CheckSection, SkippedChecks, TreeDepth};
use super::check_tally::CheckTally;
use super::check_tree::{CheckTree, TreeEntry};
use super::checks_panel::{CheckContent, CheckOrigin, ChecksPanel, ShownChecks};
use crate::button::Button;
use crate::display_date::{age_since, unix_now, unix_seconds};
use crate::fold_chevron::{FOLD_CHEVRON_SIZE, FoldChevron, FoldState};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::short_sha::short_sha;
use crate::theme;

/// One breath of the pulsing "Reading" status, as in the top bar.
const PULSE_PERIOD: Duration = Duration::from_millis(1200);
/// Diameter of the mark beside the verdict.
const VERDICT_MARK_SIZE: f32 = 18.;
/// Diameter of a row's mark.
const ROW_MARK_SIZE: f32 = 14.;
/// Height of a tree row.
const TREE_ROW_HEIGHT: f32 = 26.;
/// Indent per tree level.
const TREE_INDENT: f32 = 12.;
/// Left padding of a tree row at the top level.
const TREE_PADDING: f32 = 6.;
/// Height of the outcome bar under the verdict.
const OUTCOME_BAR_HEIGHT: f32 = 4.;
/// Size of a section heading's text.
const HEADING_TEXT_SIZE: f32 = 10.;
/// Height of a section heading's line, so headings with and without the
/// skipped switch take the same room.
const HEADING_ROW_HEIGHT: f32 = 22.;
/// Seconds in a minute and an hour, for durations.
const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;

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
                    theme::error(),
                ),
                (None, true) => div()
                    .p_3()
                    .child(render_pulsing_status(
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
        let summary = match &shown.content {
            CheckContent::Tree(tree) => Summary::of_tree(tree),
            CheckContent::NotPermitted => Summary::of_rollup(shown),
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
                    .child(div().pt(px(1.)).child(CheckOutcomeMark::new(
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
                render_inline_note(format!("Refresh failed: {error}"), theme::error())
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
                .child(render_pulsing_status("checks-reading", "Reading…"))
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
        let CheckContent::Tree(tree) = &shown.content else {
            return None;
        };
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
        let Some(CheckContent::Tree(tree)) = self.shown().map(|shown| &shown.content) else {
            return div().into_any_element();
        };
        let Some(row) = self.rows().get(index).copied() else {
            return div().into_any_element();
        };
        match row {
            CheckRow::Heading(section) => self.render_heading(section, tree, cx),
            CheckRow::Flagged(check) => render_flagged_row(tree, check, cx),
            CheckRow::Workflow(workflow) => self.render_workflow_row(index, tree, workflow, cx),
            CheckRow::Group { workflow, entry } => {
                self.render_group_row(index, tree, workflow, entry, cx)
            }
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
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(HEADING_TEXT_SIZE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme::subtle())
                            .child(title.to_uppercase()),
                    )
                    .children(skipped_switch),
            )
            .into_any_element()
    }

    /// A workflow: chevron, its outcome as one, name, and its progress.
    fn render_workflow_row(
        &self,
        row_index: usize,
        tree: &CheckTree,
        workflow_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(workflow) = tree.workflows().get(workflow_index) else {
            return div().into_any_element();
        };
        let name = workflow.name.clone();
        let is_unfolded = self.unfolded().contains(&name, None);
        render_node_row(NodeRow {
            id: ("check-workflow", row_index).into(),
            indent: 0.,
            name: workflow.name.clone().into(),
            tally: workflow.tally,
            required: workflow.required,
            fold: FoldState::of(!is_unfolded),
            weight: FontWeight::MEDIUM,
        })
        .on_click(cx.listener(move |panel, _, _, cx| panel.toggle_fold(&name, None, cx)))
        .into_any_element()
    }

    /// A group of a workflow: indented, chevron, outcome, name, progress.
    fn render_group_row(
        &self,
        row_index: usize,
        tree: &CheckTree,
        workflow_index: usize,
        entry_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(workflow) = tree.workflows().get(workflow_index) else {
            return div().into_any_element();
        };
        let Some(TreeEntry::Group(group)) = workflow.entries.get(entry_index) else {
            return div().into_any_element();
        };
        let (workflow_name, group_name) = (workflow.name.clone(), group.name.clone());
        let is_unfolded = self.unfolded().contains(&workflow_name, Some(&group_name));
        render_node_row(NodeRow {
            id: ("check-group", row_index).into(),
            indent: TREE_INDENT,
            name: group.name.clone().into(),
            tally: group.tally,
            required: group.required,
            fold: FoldState::of(!is_unfolded),
            weight: FontWeight::NORMAL,
        })
        .on_click(cx.listener(move |panel, _, _, cx| {
            panel.toggle_fold(&workflow_name, Some(&group_name), cx);
        }))
        .into_any_element()
    }
}

/// What the summary says about the checks.
struct Summary {
    outcome: Option<CheckOutcome>,
    verdict: String,
    counts: String,
    /// The outcome counts the bar shows; none when single checks are unknown.
    bar: Option<CheckTally>,
    /// The required checks' counts, when there are any.
    required: Option<CheckTally>,
}

impl Summary {
    /// The summary of checks read one by one.
    fn of_tree(tree: &CheckTree) -> Self {
        let tally = *tree.tally();
        let outcome = tally.outcome();
        let verdict = match outcome {
            None => "No checks on this commit".to_owned(),
            Some(CheckOutcome::Failed) => counted(
                tally.count(CheckOutcome::Failed),
                "check failed",
                "checks failed",
            ),
            Some(CheckOutcome::Cancelled) => {
                counted(tally.broken(), "check cancelled", "checks cancelled")
            }
            Some(CheckOutcome::Running | CheckOutcome::Queued) => counted(
                tally.pending(),
                "check still running",
                "checks still running",
            ),
            Some(CheckOutcome::Expected) => counted(
                tally.count(CheckOutcome::Expected),
                "required check not reported",
                "required checks not reported",
            ),
            Some(CheckOutcome::Passed | CheckOutcome::Neutral) => "All checks passed".to_owned(),
            Some(CheckOutcome::Skipped) => "Every check was skipped".to_owned(),
        };
        let counts = tally
            .nonzero()
            .map(|(outcome, count)| format!("{count} {}", outcome_word(outcome)))
            .collect::<Vec<_>>()
            .join(" · ");
        Self {
            outcome,
            verdict,
            counts,
            bar: (tally.ran() > 0).then_some(tally),
            required: (tree.required().total() > 0).then_some(*tree.required()),
        }
    }

    /// The summary when only GitHub's overall verdict and count are known.
    fn of_rollup(shown: &ShownChecks) -> Self {
        let verdict = match shown.rollup {
            None => "No checks on this commit",
            Some(CheckOutcome::Failed | CheckOutcome::Cancelled) => "Checks are failing",
            Some(CheckOutcome::Running | CheckOutcome::Queued | CheckOutcome::Expected) => {
                "Checks are running"
            }
            Some(CheckOutcome::Passed | CheckOutcome::Neutral | CheckOutcome::Skipped) => {
                "Checks passed"
            }
        };
        Self {
            outcome: shown.rollup,
            verdict: verdict.to_owned(),
            counts: counted(shown.total_count, "check", "checks"),
            bar: None,
            required: None,
        }
    }

    /// The summary of a commit on which nothing reported (no checks, or
    /// only required ones waiting) says why, when a merge conflict is the reason.
    fn with_conflict(self, conflict: Option<&MergeConflict>) -> Self {
        match (self.outcome, conflict) {
            (None | Some(CheckOutcome::Expected), Some(conflict)) => Self {
                outcome: Some(CheckOutcome::Failed),
                verdict: "No checks: merge conflict".to_owned(),
                counts: conflict_sentence(conflict),
                ..self
            },
            _ => self,
        }
    }
}

/// The parts of a workflow or group row.
struct NodeRow {
    id: gpui::ElementId,
    indent: f32,
    name: SharedString,
    tally: CheckTally,
    /// The required checks inside, which the row's pill counts.
    required: CheckTally,
    fold: FoldState,
    weight: FontWeight,
}

/// A foldable row: chevron, outcome mark, name, a pill when required
/// checks are inside, then its progress on the right: how many failed,
/// else how many of those that ran are done.
fn render_node_row(row: NodeRow) -> Stateful<Div> {
    let (progress, color) = match (row.tally.broken(), row.tally.ran()) {
        (0, 0) => ("skipped".to_owned(), theme::subtle()),
        (0, ran) => (format!("{}/{ran}", row.tally.finished()), theme::subtle()),
        (broken, _) => (format!("{broken} failed"), theme::danger()),
    };
    tree_row(row.id, row.indent)
        .child(FoldChevron::new("fold", row.fold))
        .child(CheckOutcomeMark::new(
            "mark",
            row.tally.outcome().unwrap_or(CheckOutcome::Neutral),
            ROW_MARK_SIZE,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(row.weight)
                .text_color(theme::text())
                .child(row.name),
        )
        .children(render_required_count(&row.required))
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(color)
                .child(progress),
        )
}

/// A check in the tree: indented past the chevrons, its mark, its name in
/// its group, how long it took (or runs); a click opens its page.
fn render_job_row(
    tree: &CheckTree,
    index: usize,
    depth: TreeDepth,
    cx: &mut Context<ChecksPanel>,
) -> AnyElement {
    let Some(check) = tree.check(index) else {
        return div().into_any_element();
    };
    let indent = match depth {
        TreeDepth::InWorkflow => TREE_INDENT,
        TreeDepth::InGroup => 2. * TREE_INDENT,
    };
    let is_skipped = check.outcome == CheckOutcome::Skipped;
    tree_row(("check-job", index).into(), indent)
        .child(div().flex_none().w(px(FOLD_CHEVRON_SIZE)))
        .child(CheckOutcomeMark::new("mark", check.outcome, ROW_MARK_SIZE))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(if is_skipped {
                    theme::subtle()
                } else {
                    theme::text()
                })
                .child(tree.job_name(index).to_owned()),
        )
        .children(render_required_tag(check))
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(theme::subtle())
                .child(timing(check, unix_now())),
        )
        .map(|row| with_details_link(row, check, cx))
        .into_any_element()
}

/// A failed or pending check above the tree: its mark, name and where it
/// sits (`Workflow › Group`) under it, how long it ran; a click opens its page.
fn render_flagged_row(tree: &CheckTree, index: usize, cx: &mut Context<ChecksPanel>) -> AnyElement {
    let Some(check) = tree.check(index) else {
        return div().into_any_element();
    };
    div()
        .id(("check-flagged", index))
        .flex()
        .items_start()
        .gap_2()
        .px_3()
        .py_1()
        .text_sm()
        .hover(|style| style.bg(theme::hover()))
        .child(
            div()
                .pt(px(2.))
                .child(CheckOutcomeMark::new("mark", check.outcome, ROW_MARK_SIZE)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_color(theme::text())
                        .child(tree.job_name(index).to_owned()),
                )
                .child(
                    div()
                        .truncate()
                        .text_xs()
                        .text_color(theme::subtle())
                        .child(tree.path_of(index)),
                ),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap_1p5()
                .pt(px(2.))
                .children(render_required_tag(check))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::subtle())
                        .child(timing(check, unix_now())),
                ),
        )
        .map(|row| with_details_link(row, check, cx))
        .into_any_element()
}

/// The frame of a tree row: one line, indented, hovering.
fn tree_row(id: gpui::ElementId, indent: f32) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_1p5()
        .h(px(TREE_ROW_HEIGHT))
        .pl(px(TREE_PADDING + indent))
        .pr_3()
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(theme::hover()))
}

/// Makes `row` open the check's page on GitHub (or its app) when clicked;
/// only `http(s)` links open, since the URL comes from GitHub's data.
fn with_details_link(
    row: Stateful<Div>,
    check: &StatusCheck,
    cx: &mut Context<ChecksPanel>,
) -> Stateful<Div> {
    let Some(url) = check
        .details_url
        .clone()
        .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
    else {
        return row.cursor_default();
    };
    row.cursor_pointer()
        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
}

/// A small `Required` pill for a check branch protection requires, in the
/// danger color when it failed or was cancelled (as its workflow's pill).
fn render_required_tag(check: &StatusCheck) -> Option<impl IntoElement> {
    let state = match check.outcome {
        CheckOutcome::Failed | CheckOutcome::Cancelled => RequiredState::Broken,
        _ => RequiredState::Fine,
    };
    check
        .is_required
        .then(|| render_required_pill("Required".to_owned(), state))
}

/// The pill of a workflow or group holding required checks, so a folded
/// row shows them: `Required` for one, `3 required` for more, in the
/// danger color while one of them failed or was cancelled.
fn render_required_count(required: &CheckTally) -> Option<impl IntoElement> {
    let label = match required.total() {
        0 => return None,
        1 => "Required".to_owned(),
        count => format!("{count} required"),
    };
    let state = match required.broken() {
        0 => RequiredState::Fine,
        _ => RequiredState::Broken,
    };
    Some(render_required_pill(label, state))
}

/// Whether the required checks a pill stands for are failing.
#[derive(Clone, Copy)]
enum RequiredState {
    Fine,
    Broken,
}

/// The pill marking required checks.
fn render_required_pill(label: String, state: RequiredState) -> impl IntoElement {
    let (border, text) = match state {
        RequiredState::Fine => (theme::border_strong(), theme::muted()),
        RequiredState::Broken => (theme::danger(), theme::danger()),
    };
    div()
        .flex_none()
        .px_1p5()
        .rounded_full()
        .border_1()
        .border_color(border)
        .text_size(px(HEADING_TEXT_SIZE))
        .text_color(text)
        .child(label)
}

/// The bar of outcomes that ran, each segment as wide as its share.
fn render_outcome_bar(tally: CheckTally) -> impl IntoElement {
    let ran = tally.ran();
    div()
        .flex()
        .gap(px(1.))
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

/// A status that pulses while it lasts, in the accent color.
fn render_pulsing_status(id: &'static str, text: &'static str) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(theme::accent())
        .child(text)
        .with_animation(
            id,
            Animation::new(PULSE_PERIOD)
                .repeat()
                .with_easing(pulsating_between(0.4, 1.)),
            Styled::opacity,
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

/// `This pull request conflicts with develop.`, or the layer below that does.
fn conflict_sentence(conflict: &MergeConflict) -> String {
    match conflict.location {
        ConflictLocation::Shown => {
            format!("This pull request conflicts with {}.", conflict.base_ref)
        }
        ConflictLocation::LayerBelow => format!(
            "#{}, lower in this stack, conflicts with {}.",
            conflict.number, conflict.base_ref
        ),
    }
}

/// What a merge conflict means for the checks: none run without a merge
/// commit, so none start until it is resolved. With checks shown, it says
/// which pull request conflicts too (the summary says so only without).
fn conflict_note(shown: &ShownChecks) -> Option<String> {
    let conflict = shown.conflict.as_ref()?;
    let has_checks = match &shown.content {
        CheckContent::Tree(tree) => tree.tally().total() > tree.expected().len(),
        CheckContent::NotPermitted => shown.rollup.is_some(),
    };
    let fix = match conflict.location {
        ConflictLocation::Shown => "Resolve it".to_owned(),
        ConflictLocation::LayerBelow => format!("Resolve it in #{}", conflict.number),
    };
    let note = if has_checks {
        format!(
            "{} GitHub starts no pull request workflows while it cannot build the \
             merge commit, so new pushes run no checks. {fix} first.",
            conflict_sentence(conflict)
        )
    } else {
        format!(
            "GitHub starts no pull request workflows while it cannot build the merge \
             commit. {fix} and the checks run."
        )
    };
    Some(note)
}

/// What the token's access leaves out, when it leaves something out.
fn access_note(shown: &ShownChecks) -> Option<&'static str> {
    match (&shown.content, shown.origin) {
        (CheckContent::NotPermitted, _) => Some(
            "This GitHub token may read only the overall state. To list the checks, \
             give it read access to Actions and Commit statuses.",
        ),
        (CheckContent::Tree(_), CheckOrigin::ActionsJobs) => Some(
            "Read from GitHub Actions: this token may not read check runs, so \
             other apps' checks are missing and required checks are matched by name.",
        ),
        (CheckContent::Tree(_), CheckOrigin::Rollup) => None,
    }
}

/// `Required: 3 of 3 passed · abc1234 · read 2m ago`, the parts known.
fn footnote(shown: &ShownChecks, required: Option<&CheckTally>) -> String {
    let read = match age_since(shown.read_at, unix_now()).as_str() {
        "now" => "read just now".to_owned(),
        age => format!("read {age} ago"),
    };
    required
        .map(|required| {
            format!(
                "Required: {} of {} passed",
                required.count(CheckOutcome::Passed),
                required.total()
            )
        })
        .into_iter()
        .chain([short_sha(&shown.head_sha).to_owned(), read])
        .collect::<Vec<_>>()
        .join(" · ")
}

/// How long a finished check took, how long a running one runs, or that it waits.
fn timing(check: &StatusCheck, now: i64) -> String {
    let started = check.started_at.as_deref().and_then(unix_seconds);
    let completed = check.completed_at.as_deref().and_then(unix_seconds);
    match (check.outcome, started, completed) {
        (CheckOutcome::Queued, _, _) => "queued".to_owned(),
        (CheckOutcome::Skipped, _, _) => String::new(),
        (CheckOutcome::Running, Some(started), _) => duration(now - started),
        (_, Some(started), Some(completed)) => duration(completed - started),
        _ => String::new(),
    }
}

/// `45s`, `3m 12s`, `1h 4m`.
fn duration(seconds: i64) -> String {
    let seconds = seconds.max(0);
    match seconds {
        0..MINUTE => format!("{seconds}s"),
        MINUTE..HOUR => format!("{}m {}s", seconds / MINUTE, seconds % MINUTE),
        _ => format!("{}h {}m", seconds / HOUR, seconds % HOUR / MINUTE),
    }
}

/// `1 check failed` or `3 checks failed`.
fn counted(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => format!("1 {one}"),
        _ => format!("{count} {many}"),
    }
}

/// The word an outcome is counted with.
fn outcome_word(outcome: CheckOutcome) -> &'static str {
    match outcome {
        CheckOutcome::Failed => "failed",
        CheckOutcome::Running => "running",
        CheckOutcome::Queued => "queued",
        CheckOutcome::Expected => "expected",
        CheckOutcome::Cancelled => "cancelled",
        CheckOutcome::Neutral => "neutral",
        CheckOutcome::Passed => "passed",
        CheckOutcome::Skipped => "skipped",
    }
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
    let as_float = |count: usize| f32::from(u16::try_from(count).unwrap_or(u16::MAX));
    match whole {
        0 => 0.,
        _ => as_float(part) / as_float(whole),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A commit without checks names the stack layer whose conflict holds them up.
    #[test]
    fn a_conflict_explains_missing_checks() {
        let conflict = MergeConflict {
            number: 36410,
            base_ref: "develop".to_owned(),
            location: ConflictLocation::LayerBelow,
        };
        let summary =
            Summary::of_tree(&CheckTree::build(Vec::new())).with_conflict(Some(&conflict));
        assert_eq!(summary.verdict, "No checks: merge conflict");
        assert_eq!(
            summary.counts,
            "#36410, lower in this stack, conflicts with develop."
        );
    }

    /// Required checks waiting on a conflicting commit are explained by the conflict.
    #[test]
    fn a_conflict_explains_waiting_required_checks() {
        let conflict = MergeConflict {
            number: 36410,
            base_ref: "develop".to_owned(),
            location: ConflictLocation::Shown,
        };
        let expected = StatusCheck {
            name: "check-lfs".to_owned(),
            source: qd_github::CheckSource::NotReported,
            outcome: CheckOutcome::Expected,
            is_required: true,
            details_url: None,
            started_at: None,
            completed_at: None,
        };
        let tree = CheckTree::build(vec![expected]);
        assert_eq!(
            Summary::of_tree(&tree).verdict,
            "1 required check not reported"
        );
        let summary = Summary::of_tree(&tree).with_conflict(Some(&conflict));
        assert_eq!(summary.verdict, "No checks: merge conflict");
    }

    /// Durations read in the largest two units.
    #[test]
    fn durations_read_shortly() {
        assert_eq!(duration(45), "45s");
        assert_eq!(duration(192), "3m 12s");
        assert_eq!(duration(3840), "1h 4m");
    }
}
