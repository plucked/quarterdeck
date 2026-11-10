//! `distinct_repositories`: repositories each once, in the order first seen.

use qd_github::RepoRef;

/// Each of `repositories` once, in the order it first appears (the sidebar's
/// groups, the Others tab's order, the Repositories settings' quick picks).
pub fn distinct_repositories<'repo>(
    repositories: impl IntoIterator<Item = &'repo RepoRef>,
) -> Vec<&'repo RepoRef> {
    let mut distinct: Vec<&RepoRef> = Vec::new();
    for repo in repositories {
        if !distinct.contains(&repo) {
            distinct.push(repo);
        }
    }
    distinct
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str) -> RepoRef {
        RepoRef {
            owner: "a".into(),
            name: name.into(),
        }
    }

    /// A repository seen again keeps its first place.
    #[test]
    fn keeps_first_seen_order() {
        let seen = [repo("web"), repo("api"), repo("web"), repo("cli")];
        let names: Vec<&str> = distinct_repositories(&seen)
            .into_iter()
            .map(|repo| repo.name.as_str())
            .collect();
        assert_eq!(names, ["web", "api", "cli"]);
    }
}
