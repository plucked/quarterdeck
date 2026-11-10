//! `counted`: a count with its noun in the singular or plural.

use std::fmt::Display;

/// `count` followed by `one` when it is 1, else by `many`: `1 check`,
/// `3 checks`, `0 lines`. Both forms are given, since English plurals are
/// not always an added `s` (`1 check failed`, `3 checks failed`).
pub fn counted<Count>(count: Count, one: &str, many: &str) -> String
where
    Count: Copy + Display + PartialEq + From<u8>,
{
    let noun = if count == Count::from(1) { one } else { many };
    format!("{count} {noun}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One takes the singular, every other count the plural.
    #[test]
    fn picks_the_noun_by_count() {
        assert_eq!(counted(1_usize, "line", "lines"), "1 line");
        assert_eq!(counted(0_u32, "line", "lines"), "0 lines");
        assert_eq!(
            counted(3_usize, "check failed", "checks failed"),
            "3 checks failed"
        );
    }
}
