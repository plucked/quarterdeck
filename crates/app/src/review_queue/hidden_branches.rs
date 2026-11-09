use qd_cache::{BranchSide, HiddenBranchRule};
use qd_github::PullRequest;

use super::queue_lists::QueueLists;

/// Whether one of `rules` hides `pull_request`: its branch on the rule's
/// side matches the rule's pattern. A pull request whose branches are not
/// known yet is never hidden.
pub fn is_hidden_by_branch(rules: &[HiddenBranchRule], pull_request: &PullRequest) -> bool {
    let Some(branches) = &pull_request.branches else {
        return false;
    };
    rules.iter().any(|rule| {
        let matches = |branch: &str| branch_pattern_matches(&rule.pattern, branch);
        match rule.side {
            BranchSide::Head => matches(&branches.head_ref),
            BranchSide::Base => matches(&branches.base_ref),
            BranchSide::Either => matches(&branches.head_ref) || matches(&branches.base_ref),
        }
    })
}

/// `lists` without the pull requests `rules` hide, in every tab.
pub fn without_hidden_branches(lists: &QueueLists, rules: &[HiddenBranchRule]) -> QueueLists {
    let keep = |list: &[PullRequest]| -> Vec<PullRequest> {
        list.iter()
            .filter(|pull_request| !is_hidden_by_branch(rules, pull_request))
            .cloned()
            .collect()
    };
    QueueLists {
        authored: keep(&lists.authored),
        review_requested: keep(&lists.review_requested),
        others: keep(&lists.others),
    }
}

/// Whether `branch` matches the wildcard `pattern` as a whole: `*` stands
/// for any run of characters (`/` included, none too), `?` for exactly one,
/// every other character for itself (case matters, as in git).
pub fn branch_pattern_matches(pattern: &str, branch: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let branch: Vec<char> = branch.chars().collect();
    let (mut pattern_index, mut branch_index) = (0, 0);
    // Where the last `*` was, and where in the branch its run ended so far.
    let mut last_star: Option<(usize, usize)> = None;
    while branch_index < branch.len() {
        match pattern.get(pattern_index) {
            // A `*` first matches nothing; a later mismatch lets it take one more character.
            Some('*') => {
                last_star = Some((pattern_index, branch_index));
                pattern_index += 1;
            }
            Some(&wanted) if wanted == '?' || wanted == branch[branch_index] => {
                pattern_index += 1;
                branch_index += 1;
            }
            _ => match last_star {
                Some((star_index, run_end)) => {
                    last_star = Some((star_index, run_end + 1));
                    pattern_index = star_index + 1;
                    branch_index = run_end + 1;
                }
                None => return false,
            },
        }
    }
    // The branch is used up: only `*`s may be left of the pattern.
    pattern[pattern_index..]
        .iter()
        .all(|&character| character == '*')
}

#[cfg(test)]
mod tests {
    use super::*;
    use qd_github::{PullRequestBranches, RepoRef, User};

    fn pull_request(head_ref: &str, base_ref: &str) -> PullRequest {
        PullRequest {
            id: 1,
            number: 1,
            repo: RepoRef {
                owner: "o".into(),
                name: "r".into(),
            },
            title: String::new(),
            body: None,
            author: User {
                login: "octo".into(),
                avatar_url: String::new(),
            },
            html_url: String::new(),
            draft: false,
            created_at: String::new(),
            updated_at: String::new(),
            labels: Vec::new(),
            stack: None,
            branches: Some(PullRequestBranches {
                head_ref: head_ref.into(),
                base_ref: base_ref.into(),
            }),
        }
    }

    fn rule(pattern: &str, side: BranchSide) -> HiddenBranchRule {
        HiddenBranchRule {
            pattern: pattern.into(),
            side,
        }
    }

    /// `*` takes any run (slashes and nothing included), `?` one character, the rest must match exactly.
    #[test]
    fn matches_wildcards() {
        assert!(branch_pattern_matches("feat/something*", "feat/something"));
        assert!(branch_pattern_matches(
            "feat/something*",
            "feat/something-else/deep"
        ));
        assert!(branch_pattern_matches("*/legacy", "team/a/legacy"));
        assert!(branch_pattern_matches("release/?.x", "release/4.x"));
        assert!(branch_pattern_matches("a*b*c", "aXbYbZc"));
        assert!(branch_pattern_matches("*", "anything"));
        assert!(!branch_pattern_matches("feat/something*", "feat/other"));
        assert!(!branch_pattern_matches(
            "feat/something*",
            "x-feat/something"
        ));
        assert!(!branch_pattern_matches("release/?.x", "release/10.x"));
        assert!(!branch_pattern_matches("main", "Main"));
        assert!(!branch_pattern_matches("a*b*c", "aXbYbZ"));
    }

    /// A rule looks only at its side; Either looks at both.
    #[test]
    fn rules_look_at_their_side() {
        let feature_into_main = pull_request("feat/x", "main");
        let head_rule = [rule("feat/*", BranchSide::Head)];
        let base_rule = [rule("feat/*", BranchSide::Base)];
        let either_rule = [rule("main", BranchSide::Either)];
        assert!(is_hidden_by_branch(&head_rule, &feature_into_main));
        assert!(!is_hidden_by_branch(&base_rule, &feature_into_main));
        assert!(is_hidden_by_branch(&either_rule, &feature_into_main));
        assert!(!is_hidden_by_branch(&[], &feature_into_main));
    }

    /// Without known branches nothing is hidden.
    #[test]
    fn keeps_pull_requests_without_branches() {
        let mut unknown = pull_request("feat/x", "main");
        unknown.branches = None;
        assert!(!is_hidden_by_branch(
            &[rule("*", BranchSide::Either)],
            &unknown
        ));
    }

    /// Every tab's list loses the hidden pull requests and keeps its order.
    #[test]
    fn filters_every_tab() {
        let lists = QueueLists {
            authored: vec![
                pull_request("feat/x", "main"),
                pull_request("fix/y", "main"),
            ],
            review_requested: vec![pull_request("feat/z", "main")],
            others: vec![pull_request("chore/a", "feat/x")],
        };
        let shown = without_hidden_branches(&lists, &[rule("feat/*", BranchSide::Either)]);
        assert_eq!(shown.authored.len(), 1);
        assert_eq!(
            shown.authored[0].branches.as_ref().unwrap().head_ref,
            "fix/y"
        );
        assert_eq!(shown.review_requested, Vec::<PullRequest>::new());
        assert_eq!(shown.others, Vec::<PullRequest>::new());
    }
}
