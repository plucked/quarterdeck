//! SQL text of the `qd-studio` enums a stored brief uses, beside its store
//! because another crate owns them; each `*_from_sql_text` is `None` for a
//! value the schema's CHECK would have refused.

use qd_studio::{
    CalloutTone, CodeSide, DiagramChange, EdgeStyle, MessageStyle, NodeRole, ReaderFamiliarity,
    StepWeight,
};

pub(crate) fn tone_sql_text(tone: CalloutTone) -> &'static str {
    match tone {
        CalloutTone::Note => "note",
        CalloutTone::Constraint => "constraint",
        CalloutTone::Verdict => "verdict",
        CalloutTone::Evidence => "evidence",
    }
}

pub(crate) fn tone_from_sql_text(text: &str) -> Option<CalloutTone> {
    match text {
        "note" => Some(CalloutTone::Note),
        "constraint" => Some(CalloutTone::Constraint),
        "verdict" => Some(CalloutTone::Verdict),
        "evidence" => Some(CalloutTone::Evidence),
        _ => None,
    }
}

pub(crate) fn side_sql_text(side: CodeSide) -> &'static str {
    match side {
        CodeSide::Base => "base",
        CodeSide::Head => "head",
    }
}

pub(crate) fn side_from_sql_text(text: &str) -> Option<CodeSide> {
    match text {
        "base" => Some(CodeSide::Base),
        "head" => Some(CodeSide::Head),
        _ => None,
    }
}

pub(crate) fn weight_sql_text(weight: StepWeight) -> &'static str {
    match weight {
        StepWeight::Core => "core",
        StepWeight::Supporting => "supporting",
        StepWeight::Mechanical => "mechanical",
    }
}

pub(crate) fn weight_from_sql_text(text: &str) -> Option<StepWeight> {
    match text {
        "core" => Some(StepWeight::Core),
        "supporting" => Some(StepWeight::Supporting),
        "mechanical" => Some(StepWeight::Mechanical),
        _ => None,
    }
}

pub(crate) fn role_sql_text(role: NodeRole) -> &'static str {
    match role {
        NodeRole::Actor => "actor",
        NodeRole::Owner => "owner",
        NodeRole::Transport => "transport",
        NodeRole::Store => "store",
        NodeRole::Constraint => "constraint",
        NodeRole::External => "external",
    }
}

pub(crate) fn role_from_sql_text(text: &str) -> Option<NodeRole> {
    NodeRole::ALL
        .into_iter()
        .find(|role| role_sql_text(*role) == text)
}

pub(crate) fn change_sql_text(change: DiagramChange) -> &'static str {
    match change {
        DiagramChange::Same => "same",
        DiagramChange::Added => "added",
        DiagramChange::Removed => "removed",
    }
}

pub(crate) fn change_from_sql_text(text: &str) -> Option<DiagramChange> {
    match text {
        "same" => Some(DiagramChange::Same),
        "added" => Some(DiagramChange::Added),
        "removed" => Some(DiagramChange::Removed),
        _ => None,
    }
}

pub(crate) fn edge_style_sql_text(style: EdgeStyle) -> &'static str {
    match style {
        EdgeStyle::Control => "control",
        EdgeStyle::Data => "data",
        EdgeStyle::Async => "async",
        EdgeStyle::Optional => "optional",
    }
}

pub(crate) fn edge_style_from_sql_text(text: &str) -> Option<EdgeStyle> {
    EdgeStyle::ALL
        .into_iter()
        .find(|style| edge_style_sql_text(*style) == text)
}

pub(crate) fn message_style_sql_text(style: MessageStyle) -> &'static str {
    match style {
        MessageStyle::Call => "call",
        MessageStyle::Reply => "reply",
        MessageStyle::Async => "async",
    }
}

pub(crate) fn message_style_from_sql_text(text: &str) -> Option<MessageStyle> {
    MessageStyle::ALL
        .into_iter()
        .find(|style| message_style_sql_text(*style) == text)
}

pub(crate) fn reader_sql_text(reader: ReaderFamiliarity) -> &'static str {
    match reader {
        ReaderFamiliarity::Expert => "expert",
        ReaderFamiliarity::Overview => "overview",
        ReaderFamiliarity::Newcomer => "newcomer",
    }
}

pub(crate) fn reader_from_sql_text(text: &str) -> Option<ReaderFamiliarity> {
    ReaderFamiliarity::ALL
        .into_iter()
        .find(|reader| reader_sql_text(*reader) == text)
}
