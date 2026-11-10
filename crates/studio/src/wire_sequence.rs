use serde::Deserialize;

use super::anchor_targets::AnchorTargets;
use super::diagram_checks::{index_ids, require_label};
use super::message_style::MessageStyle;
use super::sequence_lane::SequenceLane;
use super::sequence_message::SequenceMessage;
use super::studio_sequence::StudioSequence;
use super::wire_anchor::WireAnchor;
use super::wire_role::WireRole;

/// The most participants a sequence may have.
const MAX_SEQUENCE_LANES: usize = 6;
/// The most messages a sequence may have.
const MAX_SEQUENCE_MESSAGES: usize = 24;

/// A sequence as the agent writes it; messages name lanes by id.
#[derive(Deserialize)]
pub(crate) struct WireSequence {
    pub(crate) title: String,
    lanes: Vec<WireLane>,
    messages: Vec<WireMessage>,
}

#[derive(Deserialize)]
struct WireLane {
    id: String,
    label: String,
    role: WireRole,
}

#[derive(Deserialize)]
struct WireMessage {
    from: String,
    to: String,
    label: String,
    style: WireMessageStyle,
    note: Option<String>,
    anchor: Option<WireAnchor>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum WireMessageStyle {
    Call,
    Reply,
    Async,
}

impl From<WireMessageStyle> for MessageStyle {
    fn from(style: WireMessageStyle) -> Self {
        match style {
            WireMessageStyle::Call => Self::Call,
            WireMessageStyle::Reply => Self::Reply,
            WireMessageStyle::Async => Self::Async,
        }
    }
}

impl WireSequence {
    /// The sequence with lane ids turned into indexes, if every id is known
    /// and unique, every label is written, the size is within bounds and
    /// every anchor is in the diff; the reason otherwise.
    pub(crate) fn checked(self, targets: &AnchorTargets) -> Result<StudioSequence, String> {
        // Size first: a sequence too big to read is not worth checking further.
        if !(2..=MAX_SEQUENCE_LANES).contains(&self.lanes.len()) {
            return Err(format!(
                "a sequence needs 2 to {MAX_SEQUENCE_LANES} lanes, not {}",
                self.lanes.len()
            ));
        }
        if !(1..=MAX_SEQUENCE_MESSAGES).contains(&self.messages.len()) {
            return Err(format!(
                "a sequence needs 1 to {MAX_SEQUENCE_MESSAGES} messages, not {}",
                self.messages.len()
            ));
        }

        // Lane ids to indexes; a repeated id would make messages ambiguous.
        let lanes = index_ids(self.lanes.iter().map(|lane| lane.id.as_str()), "lane")?;
        let lane_index = |id: &str| {
            lanes
                .get(id)
                .copied()
                .ok_or_else(|| format!("a message names an unknown lane “{id}”"))
        };

        // Each message between known lanes, with its anchor in the diff.
        let mut messages = Vec::with_capacity(self.messages.len());
        for message in self.messages {
            require_label(&message.label, || {
                format!("the message from “{}” to “{}”", message.from, message.to)
            })?;
            messages.push(SequenceMessage {
                from: lane_index(&message.from)?,
                to: lane_index(&message.to)?,
                label: message.label.trim().to_owned(),
                style: message.style.into(),
                note: message
                    .note
                    .map(|note| note.trim().to_owned())
                    .filter(|note| !note.is_empty()),
                anchor: message
                    .anchor
                    .map(|anchor| anchor.checked(targets))
                    .transpose()?,
            });
        }

        Ok(StudioSequence {
            title: self.title,
            lanes: self
                .lanes
                .into_iter()
                .map(|lane| SequenceLane {
                    label: lane.label.trim().to_owned(),
                    role: lane.role.into(),
                })
                .collect(),
            messages,
        })
    }
}
