use std::time::Duration;

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// A media timestamp (`value / timescale` seconds, as Core Media counts
/// time) as a `Duration`; `None` for a negative value or a timescale that is
/// not positive (an invalid or indefinite time).
pub(crate) fn media_time_to_duration(value: i64, timescale: i32) -> Option<Duration> {
    let value = u128::try_from(value).ok()?;
    let timescale = u128::try_from(timescale).ok().filter(|scale| *scale > 0)?;
    let nanos = value * NANOS_PER_SECOND / timescale;
    Some(Duration::from_nanos(u64::try_from(nanos).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 60 fps frame in a 15360 timescale lands on its exact time.
    #[test]
    fn converts_ticks_to_time() {
        assert_eq!(
            media_time_to_duration(15_360, 15_360),
            Some(Duration::from_secs(1))
        );
        assert_eq!(
            media_time_to_duration(256, 15_360),
            Some(Duration::from_nanos(16_666_666))
        );
    }

    /// Invalid times give no duration.
    #[test]
    fn rejects_invalid_times() {
        assert_eq!(media_time_to_duration(-1, 600), None);
        assert_eq!(media_time_to_duration(1, 0), None);
    }
}
