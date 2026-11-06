use std::cmp::Ordering;

use qd_update::AppVersion;

/// What a check does with the newest release it found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateDecision {
    /// Download it: it is newer than the running app and any staged update.
    Download,
    /// Keep the update already staged: nothing newer came out.
    KeepStaged,
    /// Nothing to do: the running app is the newest.
    UpToDate,
}

/// Decides on the newest release `latest`, given the running app's
/// version and the version already staged, if any.
pub fn decide_update(
    latest: AppVersion,
    running: AppVersion,
    staged: Option<AppVersion>,
) -> UpdateDecision {
    let newest_known = staged.map_or(running, |staged| staged.max(running));
    match (latest.cmp(&newest_known), staged) {
        (Ordering::Greater, _) => UpdateDecision::Download,
        (Ordering::Equal | Ordering::Less, Some(_)) => UpdateDecision::KeepStaged,
        (Ordering::Equal | Ordering::Less, None) => UpdateDecision::UpToDate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(text: &str) -> AppVersion {
        AppVersion::parse(text).unwrap()
    }

    /// A release newer than the running app is downloaded.
    #[test]
    fn downloads_a_newer_release() {
        assert_eq!(
            decide_update(version("0.2.0"), version("0.1.0"), None),
            UpdateDecision::Download
        );
    }

    /// The running version, or an older one (a pulled release), changes nothing.
    #[test]
    fn ignores_the_same_or_an_older_release() {
        for latest in ["0.1.0", "0.0.9"] {
            assert_eq!(
                decide_update(version(latest), version("0.1.0"), None),
                UpdateDecision::UpToDate
            );
        }
    }

    /// A staged update stays until a release newer than it appears.
    #[test]
    fn keeps_the_staged_update_until_a_newer_one() {
        let staged = Some(version("0.2.0"));
        assert_eq!(
            decide_update(version("0.2.0"), version("0.1.0"), staged),
            UpdateDecision::KeepStaged
        );
        assert_eq!(
            decide_update(version("0.3.0"), version("0.1.0"), staged),
            UpdateDecision::Download
        );
    }
}
