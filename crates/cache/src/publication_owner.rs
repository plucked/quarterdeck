/// The saved artifact whose typed metadata and diagnostics a publication owns.
#[derive(Clone, Copy)]
pub(crate) enum PublicationOwner {
    GuidedReview,
    Brief,
}

impl PublicationOwner {
    /// The artifact foreign-key column shared by its publication and audit tables.
    pub(crate) fn id_column(self) -> &'static str {
        match self {
            Self::GuidedReview => "filter_id",
            Self::Brief => "brief_id",
        }
    }

    /// The trusted table identifier for one artifact's durable publication receipt.
    pub(crate) fn publications(self) -> &'static str {
        match self {
            Self::GuidedReview => "guided_review_publications",
            Self::Brief => "brief_publications",
        }
    }

    /// The trusted table identifier for the bounded ordered tool audit.
    pub(crate) fn events(self) -> &'static str {
        match self {
            Self::GuidedReview => "guided_review_tool_audit",
            Self::Brief => "brief_tool_audit",
        }
    }

    /// The trusted table identifier for diagnostic codes attached to audit events.
    pub(crate) fn issues(self) -> &'static str {
        match self {
            Self::GuidedReview => "guided_review_tool_issues",
            Self::Brief => "brief_tool_issues",
        }
    }
}
