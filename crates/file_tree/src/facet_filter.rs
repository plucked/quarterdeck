use std::borrow::Borrow;
use std::collections::BTreeSet;

use crate::FacetChoice;

/// One row of filter chips: nothing picked admits every value; picked
/// values admit only themselves; hidden values never pass. Over values an
/// item has exactly one of (a file's kind or type) use [`Self::admits`];
/// over values it has any number of (a pull request's labels), [`Self::admits_any`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetFilter<T: Ord> {
    pub only: BTreeSet<T>,
    pub hidden: BTreeSet<T>,
}

impl<T: Ord> Default for FacetFilter<T> {
    fn default() -> Self {
        Self {
            only: BTreeSet::new(),
            hidden: BTreeSet::new(),
        }
    }
}

impl<T: Ord + Clone> FacetFilter<T> {
    /// Whether a file with `value` passes this row (`value` may be borrowed,
    /// such as a `&str` for a row of `String`s).
    pub fn admits<Q>(&self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        !self.hidden.contains(value) && (self.only.is_empty() || self.only.contains(value))
    }

    /// Whether an item with all of `values` passes: none of them is hidden,
    /// and one of them is picked when anything is (an item without values
    /// passes only while nothing is picked).
    pub fn admits_any<'value>(&self, values: impl IntoIterator<Item = &'value T> + Clone) -> bool
    where
        T: 'value,
    {
        let has_hidden = values
            .clone()
            .into_iter()
            .any(|value| self.hidden.contains(value));
        let has_picked = values.into_iter().any(|value| self.only.contains(value));
        !has_hidden && (self.only.is_empty() || has_picked)
    }

    pub fn choice_of(&self, value: &T) -> FacetChoice {
        match (self.only.contains(value), self.hidden.contains(value)) {
            (_, true) => FacetChoice::Hidden,
            (true, false) => FacetChoice::Only,
            (false, false) => FacetChoice::Open,
        }
    }

    /// Picks `value` (a click), or unpicks it; a hidden value is shown again and picked.
    pub fn toggle_only(&mut self, value: &T) {
        self.hidden.remove(value);
        if !self.only.remove(value) {
            self.only.insert(value.clone());
        }
    }

    /// Hides `value` (an ⌥-click), or shows it again; a picked value is unpicked.
    pub fn toggle_hidden(&mut self, value: &T) {
        self.only.remove(value);
        if !self.hidden.remove(value) {
            self.hidden.insert(value.clone());
        }
    }

    /// Whether this row narrows anything.
    pub fn is_active(&self) -> bool {
        !self.only.is_empty() || !self.hidden.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nothing picked admits all; picks admit only themselves; hidden never passes.
    #[test]
    fn picks_and_hides() {
        let mut filter: FacetFilter<u8> = FacetFilter::default();
        assert!(filter.admits(&1) && filter.admits(&2));

        filter.toggle_only(&1);
        assert!(filter.admits(&1) && !filter.admits(&2));
        assert_eq!(filter.choice_of(&1), FacetChoice::Only);

        filter.toggle_hidden(&1);
        assert!(!filter.admits(&1) && filter.admits(&2));
        assert_eq!(filter.choice_of(&1), FacetChoice::Hidden);

        filter.toggle_only(&1);
        assert_eq!(filter.choice_of(&1), FacetChoice::Only);
        filter.toggle_only(&1);
        assert!(!filter.is_active());
    }

    /// Over several values: any picked one admits, any hidden one refuses.
    #[test]
    fn admits_any_of_several_values() {
        let mut filter: FacetFilter<u8> = FacetFilter::default();
        assert!(filter.admits_any(&[]) && filter.admits_any(&[1, 2]));

        filter.toggle_only(&1);
        assert!(filter.admits_any(&[1, 2]) && !filter.admits_any(&[2]) && !filter.admits_any(&[]));

        filter.toggle_hidden(&2);
        assert!(!filter.admits_any(&[1, 2]) && filter.admits_any(&[1, 3]));
    }
}
