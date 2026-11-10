use serde::Deserialize;

use super::node_role::NodeRole;

/// A box's or lane's role as the agent writes it.
#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WireRole {
    Actor,
    Owner,
    Transport,
    Store,
    Constraint,
    External,
}

impl From<WireRole> for NodeRole {
    fn from(role: WireRole) -> Self {
        match role {
            WireRole::Actor => Self::Actor,
            WireRole::Owner => Self::Owner,
            WireRole::Transport => Self::Transport,
            WireRole::Store => Self::Store,
            WireRole::Constraint => Self::Constraint,
            WireRole::External => Self::External,
        }
    }
}
