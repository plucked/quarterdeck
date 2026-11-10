use qd_studio::StudioSequence;

use super::diagram_geometry::{DIAGRAM_MARGIN, DiagramRect};
use super::text_estimate::{LABEL_TEXT, SMALL_TEXT};
use crate::count_as_f32::count_as_f32;

/// The least distance between two lanes' lifelines.
const MIN_LANE_SPACING: f32 = 150.;
/// The most distance between two lanes' lifelines; longer labels wrap.
const MAX_LANE_SPACING: f32 = 250.;
/// Space between a message label and the lifelines on either side of it.
const LABEL_SIDE_SPACE: f32 = 14.;
/// Space between a lane header's text and its border.
pub const HEADER_PADDING: f32 = 8.;
/// Space between two lane headers.
const HEADER_GAP: f32 = 16.;
/// Space between the headers and the first message.
const FIRST_MESSAGE_GAP: f32 = 14.;
/// Space between a message's text and its arrow.
const ARROW_GAP: f32 = 5.;
/// Space between one message's arrow and the next message's text.
const MESSAGE_GAP: f32 = 12.;
/// How far right of its lifeline a self message loops.
pub const SELF_LOOP_WIDTH: f32 = 34.;
/// How tall a self message's loop is.
pub const SELF_LOOP_HEIGHT: f32 = 16.;
/// Space below the last message before the lifelines end.
const LIFELINE_TAIL: f32 = 14.;

/// Where every part of a sequence goes: lane headers and lifelines, and
/// each message's arrow, label and note. Computed when the brief is prepared.
#[derive(Debug, Clone)]
pub struct SequenceLayout {
    pub width: f32,
    pub height: f32,
    /// Each lane's header box, by lane index; its lifeline runs down from
    /// the header's centre.
    pub headers: Vec<DiagramRect>,
    pub lifeline_bottom: f32,
    /// Each message's place, by message index.
    pub messages: Vec<MessagePlace>,
}

/// Where one message is drawn.
#[derive(Debug, Clone, Copy)]
pub struct MessagePlace {
    /// The arrow's height; a self message's loop starts here.
    pub arrow_y: f32,
    pub from_x: f32,
    pub to_x: f32,
    /// The label (and note under it), above the arrow.
    pub label: DiagramRect,
    /// The note, under the label; `None` without one.
    pub note: Option<DiagramRect>,
}

/// Lays `sequence` out: lanes as evenly spaced columns, as far apart as
/// the widest label per lane it spans needs (between two bounds, and no
/// wider than `width`, the brief's column, allows; longer labels wrap),
/// headers sized to their names, and messages top to bottom, each label
/// above its arrow.
pub fn lay_out_sequence(sequence: &StudioSequence, width: f32) -> SequenceLayout {
    // Lane spacing from the labels (a label spanning several lanes shares
    // their space), at most what fits the column; longer labels wrap.
    let fitting_spacing =
        (width - 2. * DIAGRAM_MARGIN + HEADER_GAP) / count_as_f32(sequence.lanes.len());
    let spacing = sequence
        .messages
        .iter()
        .map(|message| {
            let span = message.from.abs_diff(message.to).max(1);
            SMALL_TEXT.width_of(&message.label) / count_as_f32(span) + 2. * LABEL_SIDE_SPACE
        })
        .fold(MIN_LANE_SPACING, f32::max)
        .min(MAX_LANE_SPACING)
        .min(fitting_spacing.max(MIN_LANE_SPACING));

    // Headers, all as tall as the tallest name.
    let header_width = spacing - HEADER_GAP;
    let header_text_width = header_width - 2. * HEADER_PADDING;
    let header_height = sequence
        .lanes
        .iter()
        .map(|lane| LABEL_TEXT.height_of(&lane.label, header_text_width))
        .fold(0., f32::max)
        + 2. * HEADER_PADDING;
    let lane_x = |lane: usize| DIAGRAM_MARGIN + header_width / 2. + count_as_f32(lane) * spacing;
    let headers: Vec<DiagramRect> = (0..sequence.lanes.len())
        .map(|lane| DiagramRect {
            x: lane_x(lane) - header_width / 2.,
            y: DIAGRAM_MARGIN,
            width: header_width,
            height: header_height,
        })
        .collect();

    // Messages top to bottom, each label centred over its arrow (beside
    // the lifeline for a self message).
    let mut y = DIAGRAM_MARGIN + header_height + FIRST_MESSAGE_GAP;
    let mut right = headers.last().map_or(0., DiagramRect::right);
    let messages = sequence
        .messages
        .iter()
        .map(|message| {
            let (from_x, to_x) = (lane_x(message.from), lane_x(message.to));
            let is_self = message.from == message.to;
            let span = message.from.abs_diff(message.to).max(1);
            let text_width = count_as_f32(span) * spacing - 2. * LABEL_SIDE_SPACE;
            let label_width = SMALL_TEXT.width_of(&message.label).min(text_width);
            let label_height = SMALL_TEXT.height_of(&message.label, text_width);
            let label_x = if is_self {
                from_x + LABEL_SIDE_SPACE / 2.
            } else {
                f32::midpoint(from_x, to_x) - label_width / 2.
            };
            let label = DiagramRect {
                x: label_x,
                y,
                width: label_width,
                height: label_height,
            };
            let note = message.note.as_deref().map(|note| DiagramRect {
                x: if is_self {
                    label_x
                } else {
                    f32::midpoint(from_x, to_x) - SMALL_TEXT.width_of(note).min(text_width) / 2.
                },
                y: label.bottom(),
                width: SMALL_TEXT.width_of(note).min(text_width),
                height: SMALL_TEXT.height_of(note, text_width),
            });
            let text_bottom = note.map_or(label.bottom(), |note| note.bottom());
            let arrow_y = text_bottom + ARROW_GAP;
            let loop_height = if is_self { SELF_LOOP_HEIGHT } else { 0. };
            y = arrow_y + loop_height + MESSAGE_GAP;
            right = right
                .max(label.right())
                .max(note.map_or(0., |note| note.right()))
                .max(from_x + if is_self { SELF_LOOP_WIDTH } else { 0. });
            MessagePlace {
                arrow_y,
                from_x,
                to_x,
                label,
                note,
            }
        })
        .collect();

    let lifeline_bottom = y - MESSAGE_GAP + LIFELINE_TAIL;
    SequenceLayout {
        width: right + DIAGRAM_MARGIN,
        height: lifeline_bottom + DIAGRAM_MARGIN,
        headers,
        lifeline_bottom,
        messages,
    }
}

#[cfg(test)]
mod tests {
    use qd_studio::{MessageStyle, NodeRole, SequenceLane, SequenceMessage};

    use super::super::diagram_geometry::DEFAULT_DIAGRAM_WIDTH;
    use super::*;

    /// Messages go down in order, and each arrow sits below its own label.
    #[test]
    fn stacks_messages_in_time_order() {
        let lane = |label: &str| SequenceLane {
            label: label.into(),
            role: NodeRole::Actor,
        };
        let message = |from, to| SequenceMessage {
            from,
            to,
            label: "a message with a fairly long label that has to wrap".into(),
            style: MessageStyle::Call,
            note: Some("held meanwhile".into()),
            anchor: None,
        };
        let layout = lay_out_sequence(
            &StudioSequence {
                title: "Test".into(),
                lanes: vec![lane("Queue"), lane("GitHub")],
                messages: vec![message(0, 1), message(1, 1), message(1, 0)],
            },
            DEFAULT_DIAGRAM_WIDTH,
        );
        for pair in layout.messages.windows(2) {
            assert!(pair[0].arrow_y < pair[1].label.y);
        }
        for place in &layout.messages {
            let text_bottom = place
                .note
                .map_or(place.label.bottom(), |note| note.bottom());
            assert!(text_bottom < place.arrow_y);
        }
        assert!(layout.lifeline_bottom > layout.messages[2].arrow_y);
    }
}
