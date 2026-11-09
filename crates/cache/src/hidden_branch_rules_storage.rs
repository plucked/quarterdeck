use anyhow::{Result, anyhow};
use rusqlite::params;

use crate::{BranchSide, HiddenBranchRule, UiStateStore};

impl UiStateStore {
    /// The rules hiding pull requests by branch, in the order they were added.
    pub fn hidden_branch_rules(&self) -> Result<Vec<HiddenBranchRule>> {
        let mut statement = self
            .connection()
            .prepare("SELECT pattern, side FROM hidden_branch_rules ORDER BY position")?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        // A side this version does not know makes the row unreadable.
        rows.into_iter()
            .map(|(pattern, side)| {
                let side = BranchSide::from_sql_text(&side)
                    .ok_or_else(|| anyhow!("unknown branch side {side:?}"))?;
                Ok(HiddenBranchRule { pattern, side })
            })
            .collect()
    }

    /// Replaces the rules hiding pull requests by branch with `rules`, in that order.
    pub fn put_hidden_branch_rules(&self, rules: &[HiddenBranchRule]) -> Result<()> {
        let transaction = self.connection().unchecked_transaction()?;
        transaction.execute("DELETE FROM hidden_branch_rules", [])?;
        for (position, rule) in rules.iter().enumerate() {
            transaction.execute(
                "INSERT INTO hidden_branch_rules (position, pattern, side) VALUES (?1, ?2, ?3)",
                params![
                    i64::try_from(position)?,
                    rule.pattern,
                    rule.side.as_sql_text()
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(pattern: &str, side: BranchSide) -> HiddenBranchRule {
        HiddenBranchRule {
            pattern: pattern.into(),
            side,
        }
    }

    /// The rules come back in the order they were saved, with their sides.
    #[test]
    fn keeps_the_order_and_sides() {
        let store = UiStateStore::open_in_memory().unwrap();
        assert_eq!(store.hidden_branch_rules().unwrap(), Vec::new());
        let rules = [
            rule("feat/legacy-*", BranchSide::Head),
            rule("release/*", BranchSide::Base),
            rule("dependabot/*", BranchSide::Either),
        ];
        store.put_hidden_branch_rules(&rules).unwrap();
        assert_eq!(store.hidden_branch_rules().unwrap(), rules);
        store.put_hidden_branch_rules(&rules[1..]).unwrap();
        assert_eq!(store.hidden_branch_rules().unwrap(), rules[1..]);
    }
}
