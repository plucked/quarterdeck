//! SQL text of a stored brief: the `qd-studio` enums it uses, beside its
//! store because another crate owns them, and the brief tables' own text
//! columns (a widget's kind, an anchor's slot, a scope's list, the code
//! access), written and read in different files. Each `from_sql_text` is
//! `None` for a value the schema's CHECK would have refused.

use qd_studio::{
    CalloutTone, CodeAccess, CodeSide, DiagramChange, EdgeStyle, MessageStyle, NodeRole,
    ReaderFamiliarity, StepWeight, StudioWidget, TableRole,
};

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
