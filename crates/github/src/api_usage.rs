use std::sync::{Arc, Mutex, PoisonError};

use crate::{RateLimit, RateLimitResource};

/// What the client has asked GitHub since it was created, and the budgets
/// GitHub reported. Cheap to clone; clones share the counts.
#[derive(Debug, Clone, Default)]
pub struct ApiUsage(Arc<Mutex<ApiUsageSnapshot>>);

/// The counts at one moment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ApiUsageSnapshot {
    /// Requests sent, whatever the answer.
    pub requests: u64,
    /// Requests answered `304 Not Modified`, which cost no rate limit.
    pub not_modified: u64,
    pub core: Option<RateLimit>,
    pub search: Option<RateLimit>,
}

impl ApiUsage {
    /// The counts now.
    pub fn snapshot(&self) -> ApiUsageSnapshot {
        *self.lock()
    }

    /// Counts one request and keeps the budget its response reported.
    pub(crate) fn record(
        &self,
        is_not_modified: bool,
        budget: Option<(RateLimitResource, RateLimit)>,
    ) {
        let mut usage = self.lock();
        usage.requests += 1;
        usage.not_modified += u64::from(is_not_modified);
        match budget {
            Some((RateLimitResource::Core, limit)) => usage.core = Some(limit),
            Some((RateLimitResource::Search, limit)) => usage.search = Some(limit),
            None => {}
        }
    }

    /// The counts are plain numbers, so a poisoned lock still holds valid ones.
    fn lock(&self) -> std::sync::MutexGuard<'_, ApiUsageSnapshot> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
