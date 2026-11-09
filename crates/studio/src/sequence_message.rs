use super::code_anchor::CodeAnchor;
use super::message_style::MessageStyle;

/// One message of a sequence, between lane indexes (the same lane for work
/// a participant does on its own), in time order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceMessage {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub style: MessageStyle,
    /// A short remark under the label, such as what is held meanwhile.
    pub note: Option<String>,
    /// The lines that send it; clicking the label opens them.
    pub anchor: Option<CodeAnchor>,
}
