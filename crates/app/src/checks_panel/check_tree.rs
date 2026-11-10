use std::cmp::Ordering;

use qd_github::{CheckOutcome, CheckSource, StatusCheck};

use super::check_tally::CheckTally;

/// What separates a called workflow's name from its job's in a check
/// run's name (`Unit Tests / test`).
const GROUP_SEPARATOR: &str = " / ";
/// The heading of commit statuses, which belong to no workflow.
const COMMIT_STATUSES: &str = "Commit statuses";
/// The heading of the required checks nothing reported yet.
const NOT_REPORTED: &str = "Expected";
/// Where a required check nothing reported sits, said out of the tree.
const NOT_REPORTED_PATH: &str = "Required, waiting for its status";

/// A head commit's checks arranged for reading: by workflow (or app), then
/// by the called workflow a job belongs to, each level ordered so what needs
/// attention comes first, with the tallies every level shows.
#[derive(Debug)]
pub struct CheckTree {
    checks: Vec<StatusCheck>,
    workflows: Vec<WorkflowNode>,
    tally: CheckTally,
    required: CheckTally,
    /// The failed and cancelled checks, most urgent first.
    broken: Vec<usize>,
    /// The running and queued checks, running first.
    pending: Vec<usize>,
    /// The required checks nothing reported yet.
    expected: Vec<usize>,
    /// The group each check is in, by check index.
    check_groups: Vec<Option<String>>,
}

/// One workflow (or app, or the commit statuses) and what ran in it.
#[derive(Debug)]
pub struct WorkflowNode {
    pub name: String,
    pub entries: Vec<TreeEntry>,
    pub tally: CheckTally,
    /// The outcomes of the required checks in it, so a folded row can say
    /// it holds some.
    pub required: CheckTally,
}

/// A workflow's entry: a group of jobs of one called workflow, or a job.
#[derive(Debug)]
pub enum TreeEntry {
    Group(GroupNode),
    /// A check by index, shown under its whole name.
    Job(usize),
}

/// The jobs of one called workflow (checks named `Group / job`).
#[derive(Debug)]
pub struct GroupNode {
    pub name: String,
    /// The checks by index, shown by the part after the group's name.
    pub jobs: Vec<usize>,
    pub tally: CheckTally,
    /// The outcomes of the required jobs in it.
    pub required: CheckTally,
}

impl CheckTree {
    /// Arranges `checks`: grouped by source, then by the name before
    /// ` / ` when two or more share it, each level ordered by outcome
    /// (failed first, skipped last) and then by name.
    pub fn build(checks: Vec<StatusCheck>) -> Self {
        // Sort once so every grouping below keeps the reading order.
        let mut order: Vec<usize> = (0..checks.len()).collect();
        order.sort_by(|left, right| compare_checks(&checks[*left], &checks[*right]));

        // Group by source, keeping the first appearance of each name.
        let mut sources: Vec<(String, Vec<usize>)> = Vec::new();
        for index in order {
            let name = source_name(&checks[index].source);
            match sources.iter_mut().find(|(source, _)| *source == name) {
                Some((_, members)) => members.push(index),
                None => sources.push((name.to_owned(), vec![index])),
            }
        }
        let mut workflows: Vec<WorkflowNode> = sources
            .into_iter()
            .map(|(name, members)| WorkflowNode::build(name, &members, &checks))
            .collect();
        workflows.sort_by(|left, right| {
            compare_nodes(&left.tally, &left.name, &right.tally, &right.name)
        });

        // The flat lists read in the same order as the tree.
        let flat: Vec<usize> = workflows
            .iter()
            .flat_map(WorkflowNode::check_indexes)
            .collect();
        let pick = |outcomes: &[CheckOutcome]| -> Vec<usize> {
            let mut picked: Vec<usize> = flat
                .iter()
                .copied()
                .filter(|index| outcomes.contains(&checks[*index].outcome))
                .collect();
            picked.sort_by_key(|index| checks[*index].outcome.attention_rank());
            picked
        };
        let broken = pick(&[CheckOutcome::Failed, CheckOutcome::Cancelled]);
        let pending = pick(&[CheckOutcome::Running, CheckOutcome::Queued]);
        let expected = pick(&[CheckOutcome::Expected]);

        // Note each check's group once, so rows need not search for it.
        let mut check_groups = vec![None; checks.len()];
        for group in workflows.iter().flat_map(|workflow| &workflow.entries) {
            if let TreeEntry::Group(group) = group {
                for index in &group.jobs {
                    check_groups[*index] = Some(group.name.clone());
                }
            }
        }
        Self {
            tally: checks.iter().map(|check| check.outcome).collect(),
            required: checks
                .iter()
                .filter(|check| check.is_required)
                .map(|check| check.outcome)
                .collect(),
            checks,
            workflows,
            broken,
            pending,
            expected,
            check_groups,
        }
    }

    pub fn check(&self, index: usize) -> Option<&StatusCheck> {
        self.checks.get(index)
    }

    pub fn workflows(&self) -> &[WorkflowNode] {
        &self.workflows
    }

    /// Every check's outcome counted.
    pub fn tally(&self) -> &CheckTally {
        &self.tally
    }

    /// The required checks' outcomes counted.
    pub fn required(&self) -> &CheckTally {
        &self.required
    }

    /// The failed and cancelled checks, failed first.
    pub fn broken(&self) -> &[usize] {
        &self.broken
    }

    /// The running and queued checks, running first.
    pub fn pending(&self) -> &[usize] {
        &self.pending
    }

    /// The required checks nothing reported yet.
    pub fn expected(&self) -> &[usize] {
        &self.expected
    }

    /// Where check `index` sits, for a reader who sees it out of the tree:
    /// `Workflow › Group`, the workflow alone, or that it waits for a status.
    pub fn path_of(&self, index: usize) -> String {
        let Some(check) = self.checks.get(index) else {
            return String::new();
        };
        if check.source == CheckSource::NotReported {
            return NOT_REPORTED_PATH.to_owned();
        }
        let workflow = source_name(&check.source);
        match self.group_of(index) {
            Some(group) => format!("{workflow} › {group}"),
            None => workflow.to_owned(),
        }
    }

    /// The name check `index` shows under: the part after its group's name
    /// when it is in a group, else its whole name.
    pub fn job_name(&self, index: usize) -> &str {
        let Some(check) = self.checks.get(index) else {
            return "";
        };
        match self.group_of(index) {
            Some(group) => check
                .name
                .strip_prefix(group)
                .and_then(|rest| rest.strip_prefix(GROUP_SEPARATOR))
                .unwrap_or(&check.name),
            None => &check.name,
        }
    }

    /// The name of the group check `index` is in, if any.
    fn group_of(&self, index: usize) -> Option<&str> {
        self.check_groups.get(index)?.as_deref()
    }
}

impl WorkflowNode {
    /// The workflow `name` of the checks `members` (already in reading
    /// order): a group for each name before ` / ` shared by two or more.
    fn build(name: String, members: &[usize], checks: &[StatusCheck]) -> Self {
        // Collect the members under each prefix, in reading order.
        let mut prefixes: Vec<(&str, Vec<usize>)> = Vec::new();
        let mut loose = Vec::new();
        for index in members {
            match checks[*index].name.split_once(GROUP_SEPARATOR) {
                Some((prefix, _)) => {
                    match prefixes.iter_mut().find(|(known, _)| *known == prefix) {
                        Some((_, jobs)) => jobs.push(*index),
                        None => prefixes.push((prefix, vec![*index])),
                    }
                }
                None => loose.push(*index),
            }
        }

        // A prefix of one check stays a job under its whole name.
        let mut entries: Vec<TreeEntry> = Vec::new();
        for (prefix, jobs) in prefixes {
            match jobs.as_slice() {
                [only] => entries.push(TreeEntry::Job(*only)),
                _ => entries.push(TreeEntry::Group(GroupNode {
                    name: prefix.to_owned(),
                    tally: jobs.iter().map(|index| checks[*index].outcome).collect(),
                    required: required_tally(&jobs, checks),
                    jobs,
                })),
            }
        }
        entries.extend(loose.into_iter().map(TreeEntry::Job));
        entries.sort_by(|left, right| {
            let (left_tally, left_name) = entry_key(left, checks);
            let (right_tally, right_name) = entry_key(right, checks);
            compare_nodes(&left_tally, left_name, &right_tally, right_name)
        });
        Self {
            name,
            tally: members.iter().map(|index| checks[*index].outcome).collect(),
            required: required_tally(members, checks),
            entries,
        }
    }

    /// The workflow's checks in the order its entries show them.
    fn check_indexes(&self) -> impl Iterator<Item = usize> + '_ {
        self.entries.iter().flat_map(|entry| match entry {
            TreeEntry::Group(group) => group.jobs.clone(),
            TreeEntry::Job(index) => vec![*index],
        })
    }
}

/// What a workflow's checks are headed by.
fn source_name(source: &CheckSource) -> &str {
    match source {
        CheckSource::Workflow(name) | CheckSource::App(name) => name,
        CheckSource::CommitStatus => COMMIT_STATUSES,
        CheckSource::NotReported => NOT_REPORTED,
    }
}

/// The outcomes of the required checks among `members`.
fn required_tally(members: &[usize], checks: &[StatusCheck]) -> CheckTally {
    members
        .iter()
        .map(|index| &checks[*index])
        .filter(|check| check.is_required)
        .map(|check| check.outcome)
        .collect()
}

/// The tally and name an entry is ordered by.
fn entry_key<'a>(entry: &'a TreeEntry, checks: &'a [StatusCheck]) -> (CheckTally, &'a str) {
    match entry {
        TreeEntry::Group(group) => (group.tally, &group.name),
        TreeEntry::Job(index) => {
            let check = &checks[*index];
            ([check.outcome].into_iter().collect(), &check.name)
        }
    }
}

/// Reading order of two checks: by outcome, then by name.
fn compare_checks(left: &StatusCheck, right: &StatusCheck) -> Ordering {
    left.outcome
        .attention_rank()
        .cmp(&right.outcome.attention_rank())
        .then_with(|| compare_names(&left.name, &right.name))
}

/// Reading order of two nodes: by their outcome as one, then by name.
fn compare_nodes(
    left: &CheckTally,
    left_name: &str,
    right: &CheckTally,
    right_name: &str,
) -> Ordering {
    let rank = |tally: &CheckTally| {
        tally
            .outcome()
            .map_or(u8::MAX, CheckOutcome::attention_rank)
    };
    rank(left)
        .cmp(&rank(right))
        .then_with(|| compare_names(left_name, right_name))
}

/// Names compared as people sort them: ignoring case.
fn compare_names(left: &str, right: &str) -> Ordering {
    left.to_lowercase().cmp(&right.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A workflow job named `name` with `outcome`.
    fn job(workflow: &str, name: &str, outcome: CheckOutcome) -> StatusCheck {
        StatusCheck {
            name: name.to_owned(),
            source: CheckSource::Workflow(workflow.to_owned()),
            outcome,
            is_required: false,
            details_url: None,
            started_at: None,
            completed_at: None,
        }
    }

    /// Shared prefixes become groups; a prefix of one stays a job.
    #[test]
    fn jobs_sharing_a_prefix_form_a_group() {
        let tree = CheckTree::build(vec![
            job("Tests", "Auth Tests / check-cache", CheckOutcome::Passed),
            job("Tests", "Auth Tests / run", CheckOutcome::Skipped),
            job("Tests", "Lint / lint", CheckOutcome::Passed),
            job("Tests", "Check PR Status", CheckOutcome::Passed),
        ]);
        let workflow = &tree.workflows()[0];
        let shown: Vec<&str> = workflow
            .entries
            .iter()
            .map(|entry| match entry {
                TreeEntry::Group(group) => group.name.as_str(),
                TreeEntry::Job(index) => tree.check(*index).unwrap().name.as_str(),
            })
            .collect();
        assert_eq!(shown, ["Auth Tests", "Check PR Status", "Lint / lint"]);
        assert_eq!(tree.job_name(1), "run");
        assert_eq!(tree.path_of(1), "Tests › Auth Tests");
        assert_eq!(tree.job_name(2), "Lint / lint");
        assert_eq!(tree.path_of(2), "Tests");
    }

    /// Failing workflows and entries come first; the flat lists follow the tree.
    #[test]
    fn what_needs_attention_comes_first() {
        let tree = CheckTree::build(vec![
            job("Build", "compile", CheckOutcome::Passed),
            job("Tests", "unit", CheckOutcome::Passed),
            job("Tests", "integration", CheckOutcome::Running),
            job("Tests", "lint", CheckOutcome::Failed),
            job("Tests", "docs", CheckOutcome::Queued),
        ]);
        let names: Vec<&str> = tree
            .workflows()
            .iter()
            .map(|workflow| workflow.name.as_str())
            .collect();
        assert_eq!(names, ["Tests", "Build"]);
        assert_eq!(tree.broken(), [3]);
        assert_eq!(tree.pending(), [2, 4]);
        assert_eq!(
            tree.workflows()[0].tally.outcome(),
            Some(CheckOutcome::Failed)
        );
    }

    /// Required checks are tallied apart.
    #[test]
    fn required_checks_are_tallied_apart() {
        let mut required = job("Tests", "unit", CheckOutcome::Passed);
        required.is_required = true;
        let tree = CheckTree::build(vec![required, job("Tests", "lint", CheckOutcome::Failed)]);
        assert_eq!(tree.required().total(), 1);
        assert_eq!(tree.required().outcome(), Some(CheckOutcome::Passed));
        assert_eq!(tree.tally().outcome(), Some(CheckOutcome::Failed));
    }

    /// A workflow and a group know the required checks inside them.
    #[test]
    fn nodes_count_the_required_checks_inside() {
        let mut required = job("Tests", "Unit / run", CheckOutcome::Failed);
        required.is_required = true;
        let tree = CheckTree::build(vec![
            required,
            job("Tests", "Unit / lint", CheckOutcome::Passed),
            job("Build", "compile", CheckOutcome::Passed),
        ]);
        let tests = &tree.workflows()[0];
        assert_eq!(tests.name, "Tests");
        assert_eq!(tests.required.total(), 1);
        let TreeEntry::Group(group) = &tests.entries[0] else {
            panic!("the Unit jobs form a group");
        };
        assert_eq!(group.required.broken(), 1);
        assert_eq!(tree.workflows()[1].required.total(), 0);
    }
}
