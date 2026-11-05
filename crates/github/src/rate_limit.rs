/// One rate-limit budget as GitHub reported it on the last response that
/// used it (`x-ratelimit-*` headers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimit {
    pub limit: u32,
    pub remaining: u32,
    /// When the budget refills, in seconds since the Unix epoch.
    pub resets_at: u64,
}

/// Which budget a response used: core REST calls (5000 an hour) or search
/// (30 a minute).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitResource {
    Core,
    Search,
}

impl RateLimitResource {
    /// Decodes `x-ratelimit-resource`; other budgets (GraphQL, …) are not tracked.
    pub(crate) fn from_header(value: &str) -> Option<Self> {
        match value {
            "core" => Some(Self::Core),
            "search" => Some(Self::Search),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Search => "search",
        }
    }
}
