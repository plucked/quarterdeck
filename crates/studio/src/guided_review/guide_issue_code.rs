/// Stable diagnostic categories exposed to an agent repairing a guide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideIssueCode {
    EmptyGuidance,
    EmptyItems,
    EmptyPaths,
    UnknownPath,
    DuplicateFileInBucket,
    MissingPaths,
    InvalidOrder,
    InvalidIdentifier,
    UnknownBucket,
    RevisionConflict,
    OperationConflict,
    SealedDraft,
    ResourceLimit,
}

impl GuideIssueCode {
    /// The protocol spelling is independent of Rust's variant naming.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyGuidance => "empty_guidance",
            Self::EmptyItems => "empty_items",
            Self::EmptyPaths => "empty_paths",
            Self::UnknownPath => "unknown_path",
            Self::DuplicateFileInBucket => "duplicate_file_in_bucket",
            Self::MissingPaths => "missing_paths",
            Self::InvalidOrder => "invalid_order",
            Self::InvalidIdentifier => "invalid_identifier",
            Self::UnknownBucket => "unknown_bucket",
            Self::RevisionConflict => "revision_conflict",
            Self::OperationConflict => "operation_conflict",
            Self::SealedDraft => "sealed_draft",
            Self::ResourceLimit => "resource_limit",
        }
    }
}
