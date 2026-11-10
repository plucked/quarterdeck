/// Seconds in a day, for turning the time into a date.
const SECONDS_PER_DAY: u64 = 86_400;

/// A moment as a file-name stamp (`2026-11-08-1432`) and as text
/// (`2026-11-08 14:32 UTC`).
pub(crate) struct UtcStamp {
    pub file_name: String,
    pub readable: String,
}

/// `unix_seconds` as a UTC stamp, by the civil-from-days algorithm (Howard
/// Hinnant's), so no date library is needed.
pub(crate) fn utc_stamp(unix_seconds: u64) -> UtcStamp {
    let days = i64::try_from(unix_seconds / SECONDS_PER_DAY).unwrap_or(0);
    let seconds_of_day = unix_seconds % SECONDS_PER_DAY;
    let (hour, minute) = (seconds_of_day / 3600, seconds_of_day % 3600 / 60);
    // Days since 1970-01-01 to year, month and day.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    UtcStamp {
        file_name: format!("{year:04}-{month:02}-{day:02}-{hour:02}{minute:02}"),
        readable: format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02} UTC"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unix seconds become the right UTC date and time.
    #[test]
    fn stamps_utc_dates() {
        assert_eq!(utc_stamp(0).readable, "1970-01-01 00:00 UTC");
        // 2026-11-08 14:32 UTC.
        assert_eq!(utc_stamp(1_794_148_320).file_name, "2026-11-08-1432");
        // A leap day.
        assert_eq!(utc_stamp(1_709_164_800).readable, "2024-02-29 00:00 UTC");
    }
}
