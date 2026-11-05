//! `short_date`: GitHub's ISO 8601 timestamps as people read them.

/// Month names, January first.
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

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
}
