use std::time::{SystemTime, UNIX_EPOCH};

/// Seconds since the Unix epoch; 0 if the clock is before it.
pub(crate) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .unwrap_or_default()
}
