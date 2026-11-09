/// What a diagram box stands for. The app gives each role its own color,
/// as a second cue next to the label, and lists the roles used in a legend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRole {
    /// Something that acts: a thread, task, process or user action.
    Actor,
    /// The single owner of some state.
    Owner,
    /// What carries messages: a channel, queue, socket or request.
    Transport,
    /// Stored state: a database, cache, file or table.
    Store,
    /// A limit or invariant that shapes the flow.
    Constraint,
    /// A system outside the change: a service, the OS, another team's code.
    External,
}

impl NodeRole {
    pub const ALL: [Self; 6] = [
        Self::Actor,
        Self::Owner,
        Self::Transport,
        Self::Store,
        Self::Constraint,
        Self::External,
    ];

    /// The role as the legend names it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Actor => "Actor",
            Self::Owner => "State owner",
            Self::Transport => "Transport",
            Self::Store => "Stored state",
            Self::Constraint => "Constraint",
            Self::External => "External",
        }
    }
}
