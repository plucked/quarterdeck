use anyhow::Result;
use qd_studio::{SmartBucket, SmartFilter, SmartReviewItem};
use rusqlite::{OptionalExtension, params};

use crate::cached_statements::execute_cached;
use crate::scope_columns::ScopeColumns;
use crate::sql_integers::{read_u64, sql_integer};
use crate::{ReviewScope, UiStateStore};

const SCOPE_MATCH: &str = "pr_id IS ?1 AND repo_owner IS ?2 AND repo_name IS ?3
    AND first_number IS ?4 AND last_number IS ?5";

impl UiStateStore {
    /// Replaces a scope's plan atomically, retaining the old plan on failure.
    pub fn put_smart_filter(&self, scope: &ReviewScope, filter: &SmartFilter) -> Result<()> {
        let transaction = self.connection().unchecked_transaction()?;
        write_smart_filter(&transaction, scope, filter)?;
        transaction.commit()?;
        Ok(())
    }

    /// The latest plan, even after its head moves, so it can be shown as stale.
    pub fn smart_filter(&self, scope: &ReviewScope) -> Result<Option<SmartFilter>> {
        let columns = ScopeColumns::of(scope)?;
        let row = self.connection().query_row(
            &format!("SELECT id, head_sha, base_sha, generated_at FROM smart_filters WHERE {SCOPE_MATCH}"),
            columns.as_params(), |row| Ok((row.get::<_, i64>(0)?, SmartFilter {
                head_sha: row.get(1)?, base_sha: row.get(2)?, generated_at: read_u64(row, 3)?, buckets: vec![],
            })),
        ).optional()?;
        let Some((id, mut filter)) = row else {
            return Ok(None);
        };
        let mut statement = self.connection().prepare(
            "SELECT order_id, label, description, suggest_viewed FROM smart_buckets
             WHERE filter_id = ?1 ORDER BY order_id",
        )?;
        let buckets = statement.query_map([id], |row| {
            Ok(SmartBucket {
                order: row.get(0)?,
                label: row.get(1)?,
                description: row.get(2)?,
                suggest_viewed: row.get(3)?,
                items: vec![],
            })
        })?;
        for bucket in buckets {
            let mut bucket = bucket?;
            let mut statement = self.connection().prepare("SELECT position, label, description FROM smart_review_items WHERE filter_id = ?1 AND order_id = ?2 ORDER BY position")?;
            let items = statement.query_map(params![id, bucket.order], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    SmartReviewItem {
                        label: row.get(1)?,
                        description: row.get(2)?,
                        files: vec![],
                    },
                ))
            })?;
            for item in items {
                let (position, mut item) = item?;
                let mut statement = self.connection().prepare("SELECT filename FROM smart_review_item_files WHERE filter_id = ?1 AND order_id = ?2 AND item_position = ?3 ORDER BY position")?;
                item.files = statement
                    .query_map(params![id, bucket.order, position], |row| row.get(0))?
                    .collect::<rusqlite::Result<_>>()?;
                bucket.items.push(item);
            }
            filter.buckets.push(bucket);
        }
        Ok(Some(filter))
    }
}

/// Writes a complete guide within its caller's transaction.
pub(crate) fn write_smart_filter(
    connection: &rusqlite::Connection,
    scope: &ReviewScope,
    filter: &SmartFilter,
) -> Result<()> {
    // Replace the guide header before saving its ordered child rows.
    let columns = ScopeColumns::of(scope)?;
    connection.execute(
        &format!("DELETE FROM smart_filters WHERE {SCOPE_MATCH}"),
        columns.as_params(),
    )?;
    connection.execute(
        "INSERT INTO smart_filters (pr_id, repo_owner, repo_name, first_number, last_number,
                head_sha, base_sha, generated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        columns.as_params_followed_by(&[
            &filter.head_sha,
            &filter.base_sha,
            &sql_integer(filter.generated_at)?,
        ]),
    )?;
    let id = connection.last_insert_rowid();

    // Reuse each child insert statement throughout the complete guide.
    for bucket in &filter.buckets {
        execute_cached(
            connection,
            "INSERT INTO smart_buckets (filter_id, order_id, label, description, suggest_viewed)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id,
                bucket.order,
                bucket.label,
                bucket.description,
                bucket.suggest_viewed
            ],
        )?;
        // Items and members are saved in the same transaction as their bucket.
        for (position, item) in bucket.items.iter().enumerate() {
            let position = i64::try_from(position)?;
            execute_cached(
                connection,
                "INSERT INTO smart_review_items (filter_id, order_id, position, label, description) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, bucket.order, position, item.label, item.description],
            )?;
            for (member, filename) in item.files.iter().enumerate() {
                execute_cached(
                    connection,
                    "INSERT INTO smart_review_item_files (filter_id, order_id, item_position, position, filename) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![id, bucket.order, position, i64::try_from(member)?, filename],
                )?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use qd_github::RepoRef;

    fn sample(head: &str) -> SmartFilter {
        SmartFilter {
            head_sha: head.into(),
            base_sha: Some("base".into()),
            generated_at: 123,
            buckets: vec![SmartBucket {
                order: 1,
                label: "Generated".into(),
                description: "Compiled schema output".into(),
                suggest_viewed: true,
                items: vec![SmartReviewItem {
                    label: "Generated pair".into(),
                    description: "Inspect inputs first.".into(),
                    files: vec!["generated/a.rs".into(), "generated/b.rs".into()],
                }],
            }],
        }
    }

    #[test]
    fn plans_survive_reopening_and_scopes_and_replacement_are_independent() {
        let path =
            std::env::temp_dir().join(format!("qd-smart-filter-{}.sqlite", std::process::id()));
        let layer = ReviewScope::PullRequest(1);
        let range = ReviewScope::CombinedLayers {
            repo: RepoRef {
                owner: "a".into(),
                name: "b".into(),
            },
            first_number: 1,
            last_number: 2,
        };
        {
            let store = UiStateStore::open(&path).unwrap();
            store.put_smart_filter(&layer, &sample("old")).unwrap();
            store.put_smart_filter(&range, &sample("stack")).unwrap();
            store.put_smart_filter(&layer, &sample("new")).unwrap();
            // An invalid replacement rolls back the deletion and partial inserts.
            let mut invalid = sample("bad");
            invalid.buckets.push(invalid.buckets[0].clone());
            assert!(store.put_smart_filter(&layer, &invalid).is_err());
        }
        crate::GitHubCache::open(&path).unwrap().clear().unwrap();
        let store = UiStateStore::open(&path).unwrap();
        assert_eq!(store.smart_filter(&layer).unwrap(), Some(sample("new")));
        assert_eq!(store.smart_filter(&range).unwrap(), Some(sample("stack")));
        assert_eq!(
            store.smart_filter(&ReviewScope::PullRequest(2)).unwrap(),
            None
        );
        drop(store);
        std::fs::remove_file(path).unwrap();
    }
}
