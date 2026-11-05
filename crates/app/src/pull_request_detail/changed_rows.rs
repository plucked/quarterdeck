use super::detail_row::DetailRow;

/// The part of a file's rows that a rebuild changed: from `start`, `old_len`
/// rows were replaced by `new_len`. Splicing only this part keeps the list's
/// scroll position, which gpui moves to the start of any spliced range that
/// holds the top row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChangedRows {
    pub start: usize,
    pub old_len: usize,
    pub new_len: usize,
}

impl ChangedRows {
    /// The changed part between `old` and `new`, past their common start and
    /// end. Comment rows always count as changed: the same row value can
    /// show an edited comment of another height.
    pub fn between(old: &[DetailRow], new: &[DetailRow]) -> Self {
        let is_same = |left: &DetailRow, right: &DetailRow| left == right && !is_comment_row(left);
        let prefix = old
            .iter()
            .zip(new)
            .take_while(|(left, right)| is_same(left, right))
            .count();
        let longest_suffix = old.len().min(new.len()) - prefix;
        let suffix = old
            .iter()
            .rev()
            .zip(new.iter().rev())
            .take(longest_suffix)
            .take_while(|(left, right)| is_same(left, right))
            .count();
        Self {
            start: prefix,
            old_len: old.len() - prefix - suffix,
            new_len: new.len() - prefix - suffix,
        }
    }
}

/// Whether `row` shows review comments (or the comment editor).
fn is_comment_row(row: &DetailRow) -> bool {
    matches!(
        row,
        DetailRow::CommentCard { .. } | DetailRow::Composer { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(line: usize) -> DetailRow {
        DetailRow::UnifiedLine {
            file: 0,
            hunk: 0,
            line,
        }
    }

    /// A composer inserted after a line changes only that spot.
    #[test]
    fn insertion_changes_only_its_spot() {
        let old = [line(0), line(1), line(2)];
        let new = [line(0), line(1), DetailRow::Composer { file: 0 }, line(2)];
        assert_eq!(
            ChangedRows::between(&old, &new),
            ChangedRows {
                start: 2,
                old_len: 0,
                new_len: 1
            }
        );
    }

    /// A comment card is replaced even when its row value is the same.
    #[test]
    fn comment_rows_always_change() {
        let card = DetailRow::CommentCard { file: 0, card: 0 };
        let rows = [line(0), card, line(1)];
        assert_eq!(
            ChangedRows::between(&rows, &rows),
            ChangedRows {
                start: 1,
                old_len: 1,
                new_len: 1
            }
        );
    }
}
