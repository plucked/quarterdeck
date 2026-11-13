use std::collections::HashSet;

use super::session_group::SessionGroup;

const REPOSITORY_HEADER_HEIGHT: f32 = 32.;
const SESSION_ROW_HEIGHT: f32 = 54.;

/// Lightweight list rows; repository headings never participate in destination selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SessionPickerRow {
    Repository { group: usize, count: usize },
    Session(usize),
}

impl SessionPickerRow {
    /// Emits only groups with matching sessions and counts only those visible matches.
    pub fn for_groups(groups: &[SessionGroup], visible: &[usize]) -> Vec<Self> {
        // Discovery is usable before repository metadata arrives.
        if groups.is_empty() {
            return visible.iter().copied().map(Self::Session).collect();
        }
        let visible: HashSet<_> = visible.iter().copied().collect();
        let mut rows = Vec::new();
        for (group, members) in groups.iter().enumerate() {
            let sessions: Vec<_> = members
                .sessions
                .iter()
                .copied()
                .filter(|index| visible.contains(index))
                .collect();
            if sessions.is_empty() {
                continue;
            }
            rows.push(Self::Repository {
                group,
                count: sessions.len(),
            });
            rows.extend(sessions.into_iter().map(Self::Session));
        }
        rows
    }

    pub fn height(self) -> f32 {
        match self {
            Self::Repository { .. } => REPOSITORY_HEADER_HEIGHT,
            Self::Session(_) => SESSION_ROW_HEIGHT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Search drops empty headings, keeps repository order and reports matching session counts.
    #[test]
    fn filtered_rows_keep_only_populated_groups() {
        let groups = SessionGroup::for_repositories([
            Some("/zeta".into()),
            Some("/alpha".into()),
            Some("/zeta".into()),
            None,
        ]);
        assert_eq!(
            SessionPickerRow::for_groups(&groups, &[0, 2, 3]),
            vec![
                SessionPickerRow::Repository { group: 1, count: 2 },
                SessionPickerRow::Session(0),
                SessionPickerRow::Session(2),
                SessionPickerRow::Repository { group: 2, count: 1 },
                SessionPickerRow::Session(3),
            ]
        );
        assert_eq!(SessionPickerRow::for_groups(&groups, &[]), []);
    }
}
