use qd_github::CheckOutcome;

/// Every outcome, in attention order (the index of each is its rank).
const OUTCOMES: [CheckOutcome; 8] = [
    CheckOutcome::Failed,
    CheckOutcome::Running,
    CheckOutcome::Queued,
    CheckOutcome::Expected,
    CheckOutcome::Cancelled,
    CheckOutcome::Neutral,
    CheckOutcome::Passed,
    CheckOutcome::Skipped,
];

/// How many checks of a set stand at each outcome: a workflow's, a
/// group's, or all of a commit's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CheckTally {
    counts: [usize; OUTCOMES.len()],
}

impl CheckTally {
    /// Counts one more check at `outcome`.
    pub fn add(&mut self, outcome: CheckOutcome) {
        self.counts[usize::from(outcome.attention_rank())] += 1;
    }

    /// How many checks stand at `outcome`.
    pub fn count(&self, outcome: CheckOutcome) -> usize {
        self.counts[usize::from(outcome.attention_rank())]
    }

    /// Every check counted.
    pub fn total(&self) -> usize {
        self.counts.iter().sum()
    }

    /// The checks that ran or should run: all but the skipped.
    pub fn ran(&self) -> usize {
        self.total() - self.count(CheckOutcome::Skipped)
    }

    /// The checks that ran and finished.
    pub fn finished(&self) -> usize {
        self.ran() - self.pending() - self.count(CheckOutcome::Expected)
    }

    /// Failed or cancelled: the checks that block.
    pub fn broken(&self) -> usize {
        self.count(CheckOutcome::Failed) + self.count(CheckOutcome::Cancelled)
    }

    /// Running or queued.
    pub fn pending(&self) -> usize {
        self.count(CheckOutcome::Running) + self.count(CheckOutcome::Queued)
    }

    /// The set's outcome as one: the most urgent outcome present, except
    /// that passed checks outweigh neutral ones; `None` for no checks.
    pub fn outcome(&self) -> Option<CheckOutcome> {
        let unsettled = self.broken() + self.pending() + self.count(CheckOutcome::Expected);
        if self.count(CheckOutcome::Passed) > 0 && unsettled == 0 {
            return Some(CheckOutcome::Passed);
        }
        OUTCOMES
            .into_iter()
            .find(|outcome| self.count(*outcome) > 0)
    }

    /// The outcomes with a count, in attention order.
    pub fn nonzero(&self) -> impl Iterator<Item = (CheckOutcome, usize)> + '_ {
        OUTCOMES
            .into_iter()
            .map(|outcome| (outcome, self.count(outcome)))
            .filter(|(_, count)| *count > 0)
    }
}

impl FromIterator<CheckOutcome> for CheckTally {
    fn from_iter<T: IntoIterator<Item = CheckOutcome>>(outcomes: T) -> Self {
        let mut tally = Self::default();
        for outcome in outcomes {
            tally.add(outcome);
        }
        tally
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every outcome sits at the index of its attention rank, so counts and
    /// the attention order agree.
    #[test]
    fn outcomes_follow_the_attention_rank() {
        for (index, outcome) in OUTCOMES.into_iter().enumerate() {
            assert_eq!(usize::from(outcome.attention_rank()), index, "{outcome:?}");
        }
    }

    /// One failure outweighs any number of passes.
    #[test]
    fn a_failure_decides_the_outcome() {
        let tally: CheckTally = [
            CheckOutcome::Passed,
            CheckOutcome::Running,
            CheckOutcome::Failed,
        ]
        .into_iter()
        .collect();
        assert_eq!(tally.outcome(), Some(CheckOutcome::Failed));
    }

    /// Passed and neutral checks together read as passed; skipped ones do not count.
    #[test]
    fn passes_outweigh_neutral_and_skipped() {
        let tally: CheckTally = [
            CheckOutcome::Neutral,
            CheckOutcome::Passed,
            CheckOutcome::Skipped,
        ]
        .into_iter()
        .collect();
        assert_eq!(tally.outcome(), Some(CheckOutcome::Passed));
        assert_eq!(tally.ran(), 2);
        assert_eq!(tally.finished(), 2);
    }

    /// Only skipped checks read as skipped; none as no outcome.
    #[test]
    fn skipped_and_empty_sets() {
        let skipped: CheckTally = [CheckOutcome::Skipped].into_iter().collect();
        assert_eq!(skipped.outcome(), Some(CheckOutcome::Skipped));
        assert_eq!(CheckTally::default().outcome(), None);
    }

    /// A required check not reported yet keeps the rest from reading as passed.
    #[test]
    fn an_expected_check_is_not_passed() {
        let tally: CheckTally = [CheckOutcome::Passed, CheckOutcome::Expected]
            .into_iter()
            .collect();
        assert_eq!(tally.outcome(), Some(CheckOutcome::Expected));
        assert_eq!(tally.ran(), 2);
        assert_eq!(tally.finished(), 1);
    }
}
