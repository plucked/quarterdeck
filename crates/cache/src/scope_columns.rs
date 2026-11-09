use anyhow::Result;

use crate::ReviewScope;
use crate::sql_integers::sql_integer;

/// The scope columns of a note or a brief: `pr_id`, or the repository and
/// the range's layer numbers; the others NULL.
pub(crate) struct ScopeColumns {
    pub pr_id: Option<i64>,
    pub repo_owner: Option<String>,
    pub repo_name: Option<String>,
    pub first_number: Option<i64>,
    pub last_number: Option<i64>,
}

impl ScopeColumns {
    pub fn of(scope: &ReviewScope) -> Result<Self> {
        Ok(match scope {
            ReviewScope::PullRequest(id) => Self {
                pr_id: Some(sql_integer(*id)?),
                repo_owner: None,
                repo_name: None,
                first_number: None,
                last_number: None,
            },
            ReviewScope::CombinedLayers {
                repo,
                first_number,
                last_number,
            } => Self {
                pr_id: None,
                repo_owner: Some(repo.owner.clone()),
                repo_name: Some(repo.name.clone()),
                first_number: Some(sql_integer(*first_number)?),
                last_number: Some(sql_integer(*last_number)?),
            },
        })
    }
}
