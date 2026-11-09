use qd_studio::{
    AgentProgress, BriefRecord, CodeAccess, CodeAnchor, CodeSide, ReaderFamiliarity,
    RejectedWidget, parse_studio,
};

use super::sample_diffs::SampleDiffs;
use crate::brief_view::{BriefState, PreparedBrief, PreparedWidget};

/// The sample studio: an agent's answer as Claude Code returns it, written
/// by hand to use every widget, every tone and step weight, a graph before and
/// after, a state graph with a cycle, a sequence, and three widgets
/// the check leaves out.
const SAMPLE_STUDIO: &str = include_str!("samples/sample_studio.json");
/// What the sample run cost, for the footer.
const SAMPLE_COST_USD: f64 = 0.42;
/// Characters written so far, for the writing progress sample.
const SAMPLE_CHARACTERS_WRITTEN: usize = 12_345;
/// How far the checkout came, for its progress sample.
const SAMPLE_FETCH_PERCENT: u8 = 45;
/// Reads and searches so far, for the exploring sample.
const SAMPLE_TOOL_CALLS: usize = 7;
/// The failed run's message.
const SAMPLE_FAILURE: &str = "Claude Code exited with status 1: Credit balance is too low.";

/// One status card to show: what it stands for and its state.
pub struct SampleStatus {
    pub label: &'static str,
    /// `None`: no brief started.
    pub state: Option<BriefState>,
    /// Whether the buttons that write a brief are enabled.
    pub can_write: bool,
}

/// Everything the gallery shows: every status card, and a written brief
/// with the diffs its code is quoted from.
pub struct SampleBrief {
    pub statuses: Vec<SampleStatus>,
    pub brief: PreparedBrief,
    pub diffs: SampleDiffs,
}

impl SampleBrief {
    /// Builds the sample the way a run does: the studio is checked against
    /// the sample diffs (so the widgets it leaves out are listed with the
    /// real reasons) and its Markdown parsed. An excerpt whose file is not
    /// loaded is added afterwards, since the check never lets one through.
    /// Fails when the compiled-in studio no longer has the studio's shape.
    pub fn build() -> Result<Self, String> {
        // Check and prepare the sample studio against the sample diffs.
        let diffs = SampleDiffs::build();
        let output: serde_json::Value = serde_json::from_str(SAMPLE_STUDIO)
            .map_err(|error| format!("The sample studio is not JSON: {error}"))?;
        let checked = parse_studio(output.clone(), &diffs.anchor_targets())
            .map_err(|error| format!("The sample studio has another shape: {error}"))?;
        let record = sample_record(output, checked.rejected);
        let mut brief = PreparedBrief::prepare(checked.studio, record);

        // Put the excerpt of a file that is not loaded after the other code.
        let missing = PreparedWidget::CodeExcerpt {
            caption: "Lines of a file that is not loaded (the diff changed after the brief)."
                .into(),
            anchor: CodeAnchor {
                path: "crates/github/src/github_client.rs".into(),
                side: CodeSide::Head,
                first_line: 10,
                last_line: 14,
            },
        };
        let after_code = brief
            .widgets
            .iter()
            .position(|widget| matches!(widget, PreparedWidget::Table { .. }))
            .unwrap_or(brief.widgets.len());
        insert_widget(&mut brief, after_code, missing);

        Ok(Self {
            statuses: sample_statuses(),
            brief,
            diffs,
        })
    }
}

/// A record like a run's, for the sample: what a report of the sample
/// brief holds (its answer is the sample studio; there was no prompt or
/// transcript).
fn sample_record(answer: serde_json::Value, rejected: Vec<RejectedWidget>) -> BriefRecord {
    BriefRecord {
        repository: "plucked/quarterdeck".into(),
        number: 0,
        url: "https://github.com/plucked/quarterdeck/commit/75b165e".into(),
        subject: "The brief gallery's sample, built on commit 75b165e".into(),
        layers: Vec::new(),
        head_sha: None,
        reader: ReaderFamiliarity::Newcomer,
        code: CodeAccess::Workspace,
        prompt: "(the gallery's sample has no prompt: it is written by hand)".into(),
        answer: Some(answer),
        transcript: None,
        tool_calls: Vec::new(),
        rejected,
        cost_usd: Some(SAMPLE_COST_USD),
        session_id: None,
        model: None,
        duration: std::time::Duration::ZERO,
        written_at: 0,
    }
}

/// Inserts `widget` at `index` of `brief`'s widgets, into the page whose
/// range reaches `index` (the earlier page at a boundary), moving the
/// later pages' ranges along.
fn insert_widget(brief: &mut PreparedBrief, index: usize, widget: PreparedWidget) {
    brief.widgets.insert(index, widget);
    let holder = brief
        .pages
        .iter()
        .position(|page| page.widgets.start <= index && index <= page.widgets.end);
    for (position, page) in brief.pages.iter_mut().enumerate() {
        let range = page.widgets.clone();
        if Some(position) == holder {
            page.widgets = range.start..range.end + 1;
        } else if holder.is_some_and(|holder| position > holder) {
            page.widgets = range.start + 1..range.end + 1;
        }
    }
}

/// The status card in every state it has.
fn sample_statuses() -> Vec<SampleStatus> {
    let status = |label, state, can_write| SampleStatus {
        label,
        state,
        can_write,
    };
    vec![
        status("no brief", None, true),
        status("no brief, files still loading", None, false),
        status("starting", Some(BriefState::Writing(None)), true),
        status(
            "checking out the code",
            Some(BriefState::Writing(Some(AgentProgress::FetchingCode {
                percent: Some(SAMPLE_FETCH_PERCENT),
            }))),
            true,
        ),
        status(
            "unpacking the code",
            Some(BriefState::Writing(Some(AgentProgress::UnpackingCode))),
            true,
        ),
        status(
            "started",
            Some(BriefState::Writing(Some(AgentProgress::Started))),
            true,
        ),
        status(
            "thinking",
            Some(BriefState::Writing(Some(AgentProgress::Thinking))),
            true,
        ),
        status(
            "reading the code",
            Some(BriefState::Writing(Some(AgentProgress::Exploring {
                tool_calls: SAMPLE_TOOL_CALLS,
            }))),
            true,
        ),
        status(
            "writing",
            Some(BriefState::Writing(Some(AgentProgress::Writing {
                characters: SAMPLE_CHARACTERS_WRITTEN,
            }))),
            true,
        ),
        status(
            "failed",
            Some(BriefState::Failed(SAMPLE_FAILURE.into())),
            true,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Column widths the sample's diagrams are checked at: a narrow pane,
    /// the old fixed column, a wide pane.
    const COLUMN_WIDTHS: [f32; 3] = [640., 832., 1400.];
    /// A column too narrow for any diagram: each gets its narrowest layout.
    const NARROWEST_COLUMN: f32 = 1.;

    /// The sample keeps every widget but the three meant to be left out,
    /// and covers each kind of widget, so the gallery shows them all.
    #[test]
    fn sample_shows_every_widget_and_leaves_out_three() {
        let sample = SampleBrief::build().expect("the sample studio checks");
        assert_eq!(
            sample.brief.record.rejected.len(),
            3,
            "rejected: {:?}",
            sample.brief.record.rejected
        );
        // The pages cover every widget, in order, without gaps.
        assert!(sample.brief.pages.len() > 1);
        let mut next = 0;
        for page in &sample.brief.pages {
            assert_eq!(page.widgets.start, next);
            next = page.widgets.end;
        }
        assert_eq!(next, sample.brief.widgets.len());
        let kinds: [fn(&PreparedWidget) -> bool; 10] = [
            |widget| matches!(widget, PreparedWidget::Section { .. }),
            |widget| matches!(widget, PreparedWidget::Callout { .. }),
            |widget| matches!(widget, PreparedWidget::CodeExcerpt { .. }),
            |widget| matches!(widget, PreparedWidget::CodeCompare { .. }),
            |widget| matches!(widget, PreparedWidget::Table { .. }),
            |widget| matches!(widget, PreparedWidget::Scope { .. }),
            |widget| matches!(widget, PreparedWidget::Tour { .. }),
            |widget| matches!(widget, PreparedWidget::Graph { graph, .. } if graph.has_changes()),
            |widget| matches!(widget, PreparedWidget::Graph { graph, .. } if !graph.has_changes()),
            |widget| matches!(widget, PreparedWidget::Sequence { .. }),
        ];
        for (kind, is_kind) in kinds.iter().enumerate() {
            assert!(
                sample.brief.widgets.iter().any(is_kind),
                "no widget of kind {kind}"
            );
        }
    }

    /// Every sample diagram fits the brief's column at the widths it is
    /// laid out for, from a narrow pane to a wide one, unless even its
    /// narrowest layout is wider (it then scrolls sideways).
    #[test]
    fn sample_diagrams_fit_the_column() {
        let mut sample = SampleBrief::build().expect("the sample studio checks");
        sample.brief.fit_diagrams(NARROWEST_COLUMN);
        let narrowest = diagram_widths(&sample.brief);
        for column in COLUMN_WIDTHS {
            sample.brief.fit_diagrams(column);
            for ((title, width), (_, least)) in diagram_widths(&sample.brief).iter().zip(&narrowest)
            {
                assert!(
                    *width <= column.max(*least),
                    "“{title}” is {width} px wide in a {column} px column (narrowest {least} px)"
                );
            }
        }
    }

    /// Each diagram's title and laid-out width, in widget order.
    fn diagram_widths(brief: &PreparedBrief) -> Vec<(String, f32)> {
        brief
            .widgets
            .iter()
            .filter_map(|widget| match widget {
                PreparedWidget::Graph { graph, layout } => {
                    Some((graph.title.clone(), layout.width))
                }
                PreparedWidget::Sequence { sequence, layout } => {
                    Some((sequence.title.clone(), layout.width))
                }
                _ => None,
            })
            .collect()
    }
}
