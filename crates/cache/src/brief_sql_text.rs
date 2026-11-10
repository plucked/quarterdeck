//! SQL text of a stored brief: the `qd-studio` enums it uses, beside its
//! store because another crate owns them, and the brief tables' own text
//! columns (a widget's kind, an anchor's slot, a scope's list, the code
//! access), written and read in different files. Each `from_sql_text` is
//! `None` for a value the schema's CHECK would have refused.

use qd_studio::{
    CalloutTone, CodeAccess, CodeSide, DiagramChange, EdgeStyle, MessageStyle, NodeRole,
    ReaderFamiliarity, StepWeight, StudioWidget, TableRole,
};

const TONES: [CalloutTone; 4] = [
    CalloutTone::Note,
    CalloutTone::Constraint,
    CalloutTone::Verdict,
    CalloutTone::Evidence,
];
const SIDES: [CodeSide; 2] = [CodeSide::Base, CodeSide::Head];
const WEIGHTS: [StepWeight; 3] = [
    StepWeight::Core,
    StepWeight::Supporting,
    StepWeight::Mechanical,
];
const CHANGES: [DiagramChange; 3] = [
    DiagramChange::Same,
    DiagramChange::Added,
    DiagramChange::Removed,
];

/// A `brief_widgets.kind`: which widget a row holds; a table is stored as
/// `table` or `trace` by its role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BriefWidgetKind {
    Section,
    Callout,
    CodeExcerpt,
    CodeCompare,
    Table,
    Trace,
    Scope,
    Tour,
    Graph,
    Sequence,
}

impl BriefWidgetKind {
    const ALL: [Self; 10] = [
        Self::Section,
        Self::Callout,
        Self::CodeExcerpt,
        Self::CodeCompare,
        Self::Table,
        Self::Trace,
        Self::Scope,
        Self::Tour,
        Self::Graph,
        Self::Sequence,
    ];

    /// The kind `widget` is stored as.
    pub fn of(widget: &StudioWidget) -> Self {
        match widget {
            StudioWidget::Section { .. } => Self::Section,
            StudioWidget::Callout { .. } => Self::Callout,
            StudioWidget::CodeExcerpt { .. } => Self::CodeExcerpt,
            StudioWidget::CodeCompare { .. } => Self::CodeCompare,
            StudioWidget::Table(table) => match table.role {
                TableRole::Mapping => Self::Table,
                TableRole::Trace => Self::Trace,
            },
            StudioWidget::Scope(_) => Self::Scope,
            StudioWidget::Tour(_) => Self::Tour,
            StudioWidget::Graph(_) => Self::Graph,
            StudioWidget::Sequence(_) => Self::Sequence,
        }
    }

    /// The value stored; the column's CHECK lists the same.
    pub fn as_sql_text(self) -> &'static str {
        match self {
            Self::Section => "section",
            Self::Callout => "callout",
            Self::CodeExcerpt => "code_excerpt",
            Self::CodeCompare => "code_compare",
            Self::Table => "table",
            Self::Trace => "trace",
            Self::Scope => "scope",
            Self::Tour => "tour",
            Self::Graph => "graph",
            Self::Sequence => "sequence",
        }
    }

    /// Decodes a stored value.
    pub fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_sql_text() == text)
    }
}

/// A `brief_anchors.slot`: which of a code widget's anchors a row holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnchorSlot {
    /// A code excerpt's one anchor.
    Excerpt,
    /// A code comparison's anchor before the change.
    Before,
    /// A code comparison's anchor after the change.
    After,
}

impl AnchorSlot {
    /// The value stored and asked for; the column's CHECK lists the same.
    /// Never decoded: a slot is read by asking for it.
    pub fn as_sql_text(self) -> &'static str {
        match self {
            Self::Excerpt => "excerpt",
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

/// A `brief_scope_items.list`: which of a scope's lists an item is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeList {
    Changes,
    Unchanged,
    Mechanical,
}

impl ScopeList {
    /// The value stored and asked for; the column's CHECK lists the same.
    /// Never decoded: a list is read by asking for it.
    pub fn as_sql_text(self) -> &'static str {
        match self {
            Self::Changes => "changes",
            Self::Unchanged => "unchanged",
            Self::Mechanical => "mechanical",
        }
    }
}

/// A `briefs.code_access`: whether the run had a checkout; a diff-only
/// run's reason is its own column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeAccessKind {
    Workspace,
    DiffOnly,
}

impl CodeAccessKind {
    const ALL: [Self; 2] = [Self::Workspace, Self::DiffOnly];

    /// The kind of `access`.
    pub fn of(access: &CodeAccess) -> Self {
        match access {
            CodeAccess::Workspace => Self::Workspace,
            CodeAccess::DiffOnly(_) => Self::DiffOnly,
        }
    }

    /// The value stored; the column's CHECK lists the same.
    pub fn as_sql_text(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::DiffOnly => "diff_only",
        }
    }

    /// Decodes a stored value.
    pub fn from_sql_text(text: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.as_sql_text() == text)
    }
}

/// The value stored for a callout's tone.
pub(crate) fn tone_as_sql_text(tone: CalloutTone) -> &'static str {
    match tone {
        CalloutTone::Note => "note",
        CalloutTone::Constraint => "constraint",
        CalloutTone::Verdict => "verdict",
        CalloutTone::Evidence => "evidence",
    }
}

/// Decodes the stored value of a callout's tone; `None` for anything else.
pub(crate) fn tone_from_sql_text(text: &str) -> Option<CalloutTone> {
    TONES
        .into_iter()
        .find(|tone| tone_as_sql_text(*tone) == text)
}

/// The value stored for an anchor's side.
pub(crate) fn side_as_sql_text(side: CodeSide) -> &'static str {
    match side {
        CodeSide::Base => "base",
        CodeSide::Head => "head",
    }
}

/// Decodes the stored value of an anchor's side; `None` for anything else.
pub(crate) fn side_from_sql_text(text: &str) -> Option<CodeSide> {
    SIDES
        .into_iter()
        .find(|side| side_as_sql_text(*side) == text)
}

/// The value stored for a reading-order step's weight.
pub(crate) fn weight_as_sql_text(weight: StepWeight) -> &'static str {
    match weight {
        StepWeight::Core => "core",
        StepWeight::Supporting => "supporting",
        StepWeight::Mechanical => "mechanical",
    }
}

/// Decodes the stored value of a reading-order step's weight; `None` for anything else.
pub(crate) fn weight_from_sql_text(text: &str) -> Option<StepWeight> {
    WEIGHTS
        .into_iter()
        .find(|weight| weight_as_sql_text(*weight) == text)
}

/// The value stored for a box's or lane's role.
pub(crate) fn role_as_sql_text(role: NodeRole) -> &'static str {
    match role {
        NodeRole::Actor => "actor",
        NodeRole::Owner => "owner",
        NodeRole::Transport => "transport",
        NodeRole::Store => "store",
        NodeRole::Constraint => "constraint",
        NodeRole::External => "external",
    }
}

/// Decodes the stored value of a box's or lane's role; `None` for anything else.
pub(crate) fn role_from_sql_text(text: &str) -> Option<NodeRole> {
    NodeRole::ALL
        .into_iter()
        .find(|role| role_as_sql_text(*role) == text)
}

/// The value stored for a diagram part's change.
pub(crate) fn change_as_sql_text(change: DiagramChange) -> &'static str {
    match change {
        DiagramChange::Same => "same",
        DiagramChange::Added => "added",
        DiagramChange::Removed => "removed",
    }
}

/// Decodes the stored value of a diagram part's change; `None` for anything else.
pub(crate) fn change_from_sql_text(text: &str) -> Option<DiagramChange> {
    CHANGES
        .into_iter()
        .find(|change| change_as_sql_text(*change) == text)
}

/// The value stored for an arrow's style.
pub(crate) fn edge_style_as_sql_text(style: EdgeStyle) -> &'static str {
    match style {
        EdgeStyle::Control => "control",
        EdgeStyle::Data => "data",
        EdgeStyle::Async => "async",
        EdgeStyle::Optional => "optional",
    }
}

/// Decodes the stored value of an arrow's style; `None` for anything else.
pub(crate) fn edge_style_from_sql_text(text: &str) -> Option<EdgeStyle> {
    EdgeStyle::ALL
        .into_iter()
        .find(|style| edge_style_as_sql_text(*style) == text)
}

/// The value stored for a message's style.
pub(crate) fn message_style_as_sql_text(style: MessageStyle) -> &'static str {
    match style {
        MessageStyle::Call => "call",
        MessageStyle::Reply => "reply",
        MessageStyle::Async => "async",
    }
}

/// Decodes the stored value of a message's style; `None` for anything else.
pub(crate) fn message_style_from_sql_text(text: &str) -> Option<MessageStyle> {
    MessageStyle::ALL
        .into_iter()
        .find(|style| message_style_as_sql_text(*style) == text)
}

/// The value stored for the reader's familiarity.
pub(crate) fn reader_as_sql_text(reader: ReaderFamiliarity) -> &'static str {
    match reader {
        ReaderFamiliarity::Expert => "expert",
        ReaderFamiliarity::Overview => "overview",
        ReaderFamiliarity::Newcomer => "newcomer",
    }
}

/// Decodes the stored value of the reader's familiarity; `None` for anything else.
pub(crate) fn reader_from_sql_text(text: &str) -> Option<ReaderFamiliarity> {
    ReaderFamiliarity::ALL
        .into_iter()
        .find(|reader| reader_as_sql_text(*reader) == text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every value of the brief's own enums and of the `qd-studio` enums
    /// without an `ALL` decodes from the text it is stored as.
    #[test]
    fn brief_enums_round_trip_through_sql_text() {
        for kind in BriefWidgetKind::ALL {
            assert_eq!(
                BriefWidgetKind::from_sql_text(kind.as_sql_text()),
                Some(kind)
            );
        }
        for kind in CodeAccessKind::ALL {
            assert_eq!(
                CodeAccessKind::from_sql_text(kind.as_sql_text()),
                Some(kind)
            );
        }
        for tone in TONES {
            assert_eq!(tone_from_sql_text(tone_as_sql_text(tone)), Some(tone));
        }
        for side in SIDES {
            assert_eq!(side_from_sql_text(side_as_sql_text(side)), Some(side));
        }
        for weight in WEIGHTS {
            assert_eq!(
                weight_from_sql_text(weight_as_sql_text(weight)),
                Some(weight)
            );
        }
        for change in CHANGES {
            assert_eq!(
                change_from_sql_text(change_as_sql_text(change)),
                Some(change)
            );
        }
    }
}
