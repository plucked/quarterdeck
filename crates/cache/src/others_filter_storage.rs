use anyhow::{Result, anyhow};
use qd_file_tree::FacetFilter;
use qd_github::RepoRef;
use rusqlite::params;

use crate::{OthersFilter, RepositoryOthersFilter, UiStateStore, UpdatedWithin};

/// A chip row of the filter, as stored in `others_filter_chips.facet`.
const LABEL_FACET: &str = "label";
const AUTHOR_FACET: &str = "author";
/// A chip's state, as stored in `others_filter_chips.choice`.
const ONLY_CHOICE: &str = "only";
const HIDDEN_CHOICE: &str = "hidden";

/// The columns of an `others_filters` row, read before decoding.
struct StoredOthersFilter {
    repo: RepoRef,
    hide_drafts: bool,
    hide_review_requests: bool,
    updated_within: String,
}

impl UiStateStore {
    /// Every repository's saved Others filter (only those that narrow
    /// something are saved). A row that no longer reads fails the whole
    /// read; the caller then restores none.
    pub fn others_filters(&self) -> Result<Vec<RepositoryOthersFilter>> {
        // The switches of each repository.
        let mut statement = self.connection().prepare(
            "SELECT repo_owner, repo_name, hide_drafts, hide_review_requests, updated_within
             FROM others_filters ORDER BY repo_owner, repo_name",
        )?;
        let stored = statement
            .query_map([], |row| {
                Ok(StoredOthersFilter {
                    repo: RepoRef {
                        owner: row.get(0)?,
                        name: row.get(1)?,
                    },
                    hide_drafts: row.get(2)?,
                    hide_review_requests: row.get(3)?,
                    updated_within: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // Then its chips.
        stored
            .into_iter()
            .map(|stored| {
                let updated_within = UpdatedWithin::from_sql_text(&stored.updated_within)
                    .ok_or_else(|| anyhow!("unknown updated_within {:?}", stored.updated_within))?;
                Ok(RepositoryOthersFilter {
                    filter: OthersFilter {
                        hide_drafts: stored.hide_drafts,
                        hide_review_requests: stored.hide_review_requests,
                        updated_within,
                        labels: self.chips(&stored.repo, LABEL_FACET)?,
                        authors: self.chips(&stored.repo, AUTHOR_FACET)?,
                    },
                    repo: stored.repo,
                })
            })
            .collect()
    }

    /// Saves `filter` as `repo`'s Others filter; a filter that narrows
    /// nothing removes the saved one.
    pub fn put_others_filter(&self, repo: &RepoRef, filter: &OthersFilter) -> Result<()> {
        let transaction = self.connection().unchecked_transaction()?;
        // The chips go with the row (ON DELETE CASCADE).
        transaction.execute(
            "DELETE FROM others_filters WHERE repo_owner = ?1 AND repo_name = ?2",
            params![repo.owner, repo.name],
        )?;
        if filter.is_active() {
            transaction.execute(
                "INSERT INTO others_filters
                    (repo_owner, repo_name, hide_drafts, hide_review_requests, updated_within)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    repo.owner,
                    repo.name,
                    filter.hide_drafts,
                    filter.hide_review_requests,
                    filter.updated_within.as_sql_text()
                ],
            )?;
            let chips = [
                (LABEL_FACET, &filter.labels),
                (AUTHOR_FACET, &filter.authors),
            ];
            for (facet, chip_row) in chips {
                let choices = chip_row
                    .only
                    .iter()
                    .map(|value| (value, ONLY_CHOICE))
                    .chain(chip_row.hidden.iter().map(|value| (value, HIDDEN_CHOICE)));
                for (value, choice) in choices {
                    transaction.execute(
                        "INSERT INTO others_filter_chips (repo_owner, repo_name, facet, value, choice)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![repo.owner, repo.name, facet, value, choice],
                    )?;
                }
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// The picked and hidden values of one chip row of `repo`'s filter.
    fn chips(&self, repo: &RepoRef, facet: &str) -> Result<FacetFilter<String>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT value, choice FROM others_filter_chips
             WHERE repo_owner = ?1 AND repo_name = ?2 AND facet = ?3",
        )?;
        let mut chips = FacetFilter::default();
        let rows = statement.query_map(params![repo.owner, repo.name, facet], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (value, choice) = row?;
            match choice.as_str() {
                ONLY_CHOICE => chips.only.insert(value),
                HIDDEN_CHOICE => chips.hidden.insert(value),
                _ => return Err(anyhow!("unknown chip choice {choice:?}")),
            };
        }
        Ok(chips)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo() -> RepoRef {
        RepoRef {
            owner: "o".into(),
            name: "web".into(),
        }
    }

    /// A filter round-trips; one that narrows nothing is removed.
    #[test]
    fn round_trips_and_removes_inactive_filters() {
        let store = UiStateStore::open_in_memory().unwrap();
        let mut filter = OthersFilter {
            hide_drafts: true,
            updated_within: UpdatedWithin::Week,
            ..OthersFilter::default()
        };
        filter.labels.toggle_only(&"bug".to_owned());
        filter.authors.toggle_hidden(&"dependabot[bot]".to_owned());
        store.put_others_filter(&repo(), &filter).unwrap();
        assert_eq!(
            store.others_filters().unwrap(),
            [RepositoryOthersFilter {
                repo: repo(),
                filter
            }]
        );

        store
            .put_others_filter(&repo(), &OthersFilter::default())
            .unwrap();
        assert_eq!(store.others_filters().unwrap(), Vec::new());
    }
}
