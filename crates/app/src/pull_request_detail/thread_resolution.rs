use qd_github::ReviewThread;

/// A change to a GitHub conversation's resolution the viewer may make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadResolution {
    Resolve,
    Reopen,
}

impl ThreadResolution {
    /// The change `thread` offers the viewer: resolving it while open,
    /// reopening it once resolved, each only when GitHub permits it.
    pub fn offered_for(thread: &ReviewThread) -> Option<Self> {
        if thread.is_resolved {
            thread.permissions.can_unresolve.then_some(Self::Reopen)
        } else {
            thread.permissions.can_resolve.then_some(Self::Resolve)
        }
    }

    /// The button's label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Resolve => "Resolve",
            Self::Reopen => "Reopen",
        }
    }

    /// What the review tab says while GitHub makes the change.
    pub fn working_message(self) -> &'static str {
        match self {
            Self::Resolve => "Resolving the conversation…",
            Self::Reopen => "Reopening the conversation…",
        }
    }

    /// Whether the conversation is resolved after the change.
    pub fn is_resolved_after(self) -> bool {
        match self {
            Self::Resolve => true,
            Self::Reopen => false,
        }
    }
}
