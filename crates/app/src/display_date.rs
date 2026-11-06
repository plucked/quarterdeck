//! `short_date` and `relative_age` (with `unix_now`): GitHub's ISO 8601
//! timestamps as people read them; `unix_seconds` reads one as a time.

/// Month names, January first.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Seconds in a minute, hour, day and week.
const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;
/// Ages from this many weeks on read as a date instead.
const WEEKS_SHOWN: i64 = 5;

/// How long ago `timestamp` was at `now` (Unix seconds), shortly: `now`,
/// `12m`, `3h`, `2d`, `3w`; older ones as their date, unreadable ones as given.
pub fn relative_age(timestamp: &str, now: i64) -> String {
    let Some(then) = unix_seconds(timestamp) else {
        return timestamp.to_owned();
    };
    let age = (now - then).max(0);
    match age {
        0..MINUTE => "now".to_owned(),
        MINUTE..HOUR => format!("{}m", age / MINUTE),
        HOUR..DAY => format!("{}h", age / HOUR),
        DAY..WEEK => format!("{}d", age / DAY),
        _ if age < WEEKS_SHOWN * WEEK => format!("{}w", age / WEEK),
        _ => short_date(timestamp),
    }
}

/// The current time in Unix seconds, for [`relative_age`].
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

/// `2026-10-21T09:59:57Z` (UTC, as GitHub sends) as Unix seconds; `None`
/// for anything else.
pub fn unix_seconds(timestamp: &str) -> Option<i64> {
    let field = |range: std::ops::Range<usize>| timestamp.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (field(0..4)?, field(5..7)?, field(8..10)?);
    let (hour, minute, second) = (field(11..13)?, field(14..16)?, field(17..19)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    // Days since 1970-01-01 of the civil date (Howard Hinnant's algorithm).
    let shifted_year = if month <= 2 { year - 1 } else { year };
    let era = shifted_year.div_euclid(400);
    let year_of_era = shifted_year - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(days * DAY + hour * HOUR + minute * MINUTE + second)
}

/// `2026-10-21T09:59:57Z` as `21 Oct 2026`; anything else as given.
pub fn short_date(timestamp: &str) -> String {
    let date = timestamp.get(..10).unwrap_or(timestamp);
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return timestamp.to_owned();
    };
    let month_name = month
        .parse::<usize>()
        .ok()
        .and_then(|month| MONTHS.get(month.checked_sub(1)?));
    match (month_name, day.parse::<u32>()) {
        (Some(month), Ok(day)) => format!("{day} {month} {year}"),
        (None, _) | (Some(_), Err(_)) => timestamp.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GitHub timestamp reads as day, month name and year; other text stays.
    #[test]
    fn formats_github_timestamps() {
        assert_eq!(short_date("2026-10-21T09:59:57Z"), "21 Oct 2026");
        assert_eq!(short_date("2026-01-05"), "5 Jan 2026");
        assert_eq!(short_date("soon"), "soon");
    }

    /// GitHub timestamps convert to Unix seconds.
    #[test]
    fn reads_unix_seconds() {
        assert_eq!(unix_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix_seconds("2026-11-05T14:23:10Z"), Some(1_793_888_590));
        assert_eq!(unix_seconds("2024-02-29T12:00:00Z"), Some(1_709_208_000));
        assert_eq!(unix_seconds("yesterday"), None);
    }

    /// Ages read in the largest whole unit up to a few weeks, then as a date.
    #[test]
    fn ages_read_shortly() {
        let then = "2026-11-05T12:00:00Z";
        let at = unix_seconds(then).unwrap();
        assert_eq!(relative_age(then, at + 30), "now");
        assert_eq!(relative_age(then, at + 12 * MINUTE), "12m");
        assert_eq!(relative_age(then, at + 3 * HOUR), "3h");
        assert_eq!(relative_age(then, at + 2 * DAY), "2d");
        assert_eq!(relative_age(then, at + 3 * WEEK), "3w");
        assert_eq!(relative_age(then, at + 9 * WEEK), "5 Nov 2026");
    }
}
