use super::sequence_lane::SequenceLane;
use super::sequence_message::SequenceMessage;

/// A static swimlane: participants as columns, messages as arrows in time
/// order, for concurrency and protocols.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioSequence {
    pub title: String,
    pub lanes: Vec<SequenceLane>,
    pub messages: Vec<SequenceMessage>,
}
