use std::time::Duration;

/// Seconds in one hour.
const SECONDS_PER_HOUR: u64 = 60 * 60;

/// How long cached GitHub data that is not used is kept before cleanup
/// removes it. Stored as hours in `cache_settings.ttl_hours`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CacheTtl {
    SixHours,
    #[default]
    OneDay,
    ThreeDays,
    OneWeek,
}

impl CacheTtl {
    pub const ALL: [Self; 4] = [Self::SixHours, Self::OneDay, Self::ThreeDays, Self::OneWeek];

    /// The hours stored in SQLite; the table's CHECK constraint lists the same values.
    pub(crate) fn hours(self) -> u32 {
        match self {
            Self::SixHours => 6,
            Self::OneDay => 24,
            Self::ThreeDays => 72,
            Self::OneWeek => 168,
        }
    }

    /// Decodes stored hours; `None` for any other value.
    pub(crate) fn from_hours(hours: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|ttl| ttl.hours() == hours)
    }

    pub fn duration(self) -> Duration {
        Duration::from_secs(u64::from(self.hours()) * SECONDS_PER_HOUR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every TTL decodes from the hours it is stored as.
    #[test]
    fn every_ttl_round_trips_through_hours() {
        for ttl in CacheTtl::ALL {
            assert_eq!(CacheTtl::from_hours(ttl.hours()), Some(ttl));
        }
        assert_eq!(CacheTtl::from_hours(5), None);
    }
}
