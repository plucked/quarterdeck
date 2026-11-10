use crate::{CheckOutcome, CheckSource, StatusCheck};

/// Where the required marks of read checks come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RequiredMarks {
    /// GitHub marked each check itself (`isRequired`), which is exact.
    AsRead,
    /// The checks came without marks (from GitHub Actions jobs): a check is
    /// required when its name is one the base branch requires.
    ByName,
}

/// Completes `checks` with what the base branch requires
/// (`required_contexts`, check names): marks them per `marks`, then adds
/// each required check no check reported under its name as
/// [`CheckOutcome::Expected`], so a check that never started still shows.
pub(crate) fn add_expected_checks(
    checks: &mut Vec<StatusCheck>,
    required_contexts: &[String],
    marks: RequiredMarks,
) {
    // Checks read from Actions learn which are required from the names.
    if marks == RequiredMarks::ByName {
        for check in checks.iter_mut() {
            check.is_required = required_contexts.contains(&check.name);
        }
    }

    // A required name nothing reported is waiting for its status.
    let missing: Vec<StatusCheck> = required_contexts
        .iter()
        .filter(|context| !checks.iter().any(|check| check.name == **context))
        .map(|context| StatusCheck {
            name: context.clone(),
            source: CheckSource::NotReported,
            outcome: CheckOutcome::Expected,
            is_required: true,
            details_url: None,
            started_at: None,
            completed_at: None,
        })
        .collect();
    checks.extend(missing);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A passed Actions job named `name`, not marked required.
    fn job(name: &str) -> StatusCheck {
        StatusCheck {
            name: name.to_owned(),
            source: CheckSource::Workflow("CI".to_owned()),
            outcome: CheckOutcome::Passed,
            is_required: false,
            details_url: None,
            started_at: None,
            completed_at: None,
        }
    }

    /// Required names nothing reported are added as expected.
    #[test]
    fn unreported_required_checks_are_expected() {
        let mut checks = vec![job("check-lfs"), job("lint")];
        let required = ["check-lfs".to_owned(), "Build Pipeline Result".to_owned()];
        add_expected_checks(&mut checks, &required, RequiredMarks::AsRead);
        assert_eq!(checks.len(), 3);
        assert!(!checks[0].is_required);
        assert_eq!(checks[2].name, "Build Pipeline Result");
        assert_eq!(checks[2].outcome, CheckOutcome::Expected);
        assert_eq!(checks[2].source, CheckSource::NotReported);
        assert!(checks[2].is_required);
    }

    /// Checks without marks are marked required by name.
    #[test]
    fn unmarked_checks_are_marked_by_name() {
        let mut checks = vec![job("check-lfs"), job("lint")];
        add_expected_checks(
            &mut checks,
            &["check-lfs".to_owned()],
            RequiredMarks::ByName,
        );
        assert!(checks[0].is_required);
        assert!(!checks[1].is_required);
        assert_eq!(checks.len(), 2);
    }
}
