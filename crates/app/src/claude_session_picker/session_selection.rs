/// Searchable session positions and one explicit selection in the current inventory.
/// Filtering cannot leave an invisible destination selected.
#[derive(Default)]
pub(super) struct SessionSelection {
    searchable: Vec<String>,
    visible: Vec<usize>,
    order: Vec<usize>,
    selected: Option<usize>,
}

impl SessionSelection {
    /// Installs a new inventory and drops the previous destination.
    pub fn replace(&mut self, searchable: Vec<String>, query: &str) {
        self.selected = None;
        self.order = (0..searchable.len()).collect();
        self.update_searchable(searchable, query);
    }

    /// Metadata arriving for the same rows preserves any still-visible choice.
    pub fn update_searchable(&mut self, searchable: Vec<String>, query: &str) {
        self.searchable = searchable
            .into_iter()
            .map(|text| text.to_lowercase())
            .collect();
        self.filter(query);
    }

    /// Changes navigation to repository order without changing a still-visible destination.
    pub fn set_order(&mut self, order: Vec<usize>, query: &str) {
        self.order = order;
        self.filter(query);
    }

    /// Matches every space-separated term anywhere in the name, repository, path, branch or ID.
    pub fn filter(&mut self, query: &str) {
        let query = query.to_lowercase();
        let terms: Vec<_> = query.split_whitespace().collect();
        self.visible = self
            .order
            .iter()
            .copied()
            .filter(|index| {
                self.searchable
                    .get(*index)
                    .is_some_and(|text| terms.iter().all(|term| text.contains(term)))
            })
            .collect();
        // A hidden selection must never survive behind an enabled Send button.
        if self
            .selected
            .is_some_and(|index| !self.visible.contains(&index))
        {
            self.selected = None;
        }
    }

    pub fn visible(&self) -> &[usize] {
        &self.visible
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Selects only a visible session from this inventory.
    pub fn select(&mut self, index: usize) {
        if self.visible.contains(&index) {
            self.selected = Some(index);
        }
    }

    /// The selected session's row in the filtered list.
    pub fn selected_row(&self) -> Option<usize> {
        self.selected
            .and_then(|selected| self.visible.iter().position(|index| *index == selected))
    }

    /// Advances through visible matches, stopping at the last one.
    pub fn select_next(&mut self) {
        if self.visible.is_empty() {
            return;
        }
        let row = self
            .selected_row()
            .map_or(0, |row| (row + 1).min(self.visible.len() - 1));
        self.selected = self.visible.get(row).copied();
    }

    /// Moves toward the first visible match without wrapping to the end.
    pub fn select_previous(&mut self) {
        let row = self.selected_row().unwrap_or(0).saturating_sub(1);
        self.selected = self.visible.get(row).copied();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Name and full-path terms combine regardless of case or word order.
    #[test]
    fn filters_across_name_path_and_identity() {
        let mut selection = SessionSelection::default();
        selection.replace(
            vec![
                "Éco review /projects/seed/worktree-a abc123".into(),
                "Éco review /projects/tools/worktree-b def456".into(),
            ],
            "SEED éco",
        );
        assert_eq!(selection.visible(), &[0]);
        assert_eq!(selection.selected(), None);
        selection.filter("DEF456");
        assert_eq!(selection.visible(), &[1]);
    }

    /// Hiding a selected destination disables it, and clearing the filter does not select it again.
    #[test]
    fn hidden_selection_is_cleared() {
        let mut selection = SessionSelection::default();
        selection.replace(vec!["alpha".into(), "beta".into()], "");
        selection.select(0);
        selection.filter("beta");
        assert_eq!(selection.selected(), None);
        selection.filter("");
        assert_eq!(selection.selected(), None);
        assert_eq!(selection.visible(), &[0, 1]);
    }

    /// Branch metadata updates search results without silently changing the destination.
    #[test]
    fn branch_enrichment_preserves_selection_and_exposes_new_matches() {
        let mut selection = SessionSelection::default();
        selection.replace(vec!["alpha".into(), "beta".into()], "");
        selection.select(1);
        selection.update_searchable(vec!["alpha feature/review".into(), "beta main".into()], "");
        assert_eq!(selection.selected(), Some(1));
        selection.filter("feature/review");
        assert_eq!(selection.visible(), &[0]);
        assert_eq!(selection.selected(), None);
    }

    /// Regrouping preserves the destination and makes arrows follow the displayed repository order.
    #[test]
    fn repository_order_preserves_selection_and_navigation() {
        let mut selection = SessionSelection::default();
        selection.replace(
            vec!["zeta first".into(), "alpha".into(), "zeta last".into()],
            "",
        );
        selection.select(0);
        selection.set_order(vec![1, 0, 2], "");
        assert_eq!(selection.selected(), Some(0));
        selection.select_previous();
        assert_eq!(selection.selected(), Some(1));
        selection.select_next();
        selection.select_next();
        assert_eq!(selection.selected(), Some(2));
        selection.filter("zeta");
        assert_eq!(selection.visible(), &[0, 2]);
        assert_eq!(selection.selected(), Some(2));
    }

    /// Arrow navigation and refresh cannot accidentally target a hidden or previously selected index.
    #[test]
    fn navigation_uses_filtered_positions_and_refresh_clears_selection() {
        let mut selection = SessionSelection::default();
        selection.replace(
            vec!["match a".into(), "hidden".into(), "match b".into()],
            "match",
        );
        selection.select_next();
        assert_eq!(selection.selected(), Some(0));
        selection.select_next();
        assert_eq!(selection.selected(), Some(2));
        selection.select_next();
        assert_eq!(selection.selected(), Some(2));
        selection.select_previous();
        assert_eq!(selection.selected(), Some(0));
        selection.replace(vec!["match a".into()], "match");
        assert_eq!(selection.selected(), None);
        selection.filter("absent");
        selection.select_next();
        selection.select_previous();
        assert_eq!(selection.selected(), None);
    }
}
