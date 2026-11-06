use qd_file_tree::FacetCount;

use super::COLLAPSED_CHIPS;

/// Which chips of a row show and the `+n more` / `less` link closing it:
/// collapsed, the most common chips and every picked or hidden one (so a
/// set chip never hides behind "more"); expanded, all of them.
pub struct CollapsedChipRow<'count> {
    /// The chips to draw, in the counts' order.
    pub shown: Vec<&'count FacetCount<String>>,
    /// The more / less link, when there is more than the collapsed row holds.
    pub more_label: Option<String>,
}

impl<'count> CollapsedChipRow<'count> {
    /// The row for `counts`: `most_common` are the values a collapsed row
    /// shows, `is_set` says whether a value is picked or hidden.
    pub fn of(
        counts: &'count [FacetCount<String>],
        most_common: &[&str],
        is_expanded: bool,
        is_set: impl Fn(&str) -> bool,
    ) -> Self {
        // The chips that show.
        let shown: Vec<&FacetCount<String>> = counts
            .iter()
            .filter(|count| {
                is_expanded || most_common.contains(&count.value.as_str()) || is_set(&count.value)
            })
            .collect();

        // The link: less while expanded past the collapsed size, more while some are left out.
        let hidden_count = counts.len() - shown.len();
        let more_label = match (is_expanded, hidden_count) {
            (true, _) if counts.len() > COLLAPSED_CHIPS => Some("less".to_owned()),
            (false, 1..) => Some(format!("+{hidden_count} more")),
            (true | false, _) => None,
        };
        Self { shown, more_label }
    }
}
