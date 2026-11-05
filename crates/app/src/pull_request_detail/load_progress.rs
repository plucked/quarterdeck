use gpui::SharedString;
use qc_github::DownloadProgress;

/// Bytes in a megabyte, for download sizes.
const BYTES_PER_MEGABYTE: f64 = 1_048_576.;
/// The steps a fraction is computed in, so it stays integer until the end.
const FRACTION_STEPS: u64 = 1000;

/// How far loading the shown files has come, reported from the background
/// load and shown while the files are not there yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadProgress {
    /// Reading the cache; usually over before anything is drawn.
    ReadingCache,
    /// Waiting for the stack's layers, which a combined review needs first.
    LoadingStack,
    /// Asking GitHub whether the pull request's commits changed.
    CheckingCommits,
    /// Fetching the pull request's files, page by page.
    FetchingFiles { fetched: usize, total: Option<u64> },
    /// Downloading a whole diff: a combined range of layers, or a pull
    /// request with more files than GitHub lists.
    DownloadingDiff(DownloadProgress),
    /// Building the diffs of this many files.
    BuildingDiffs { files: usize },
}

impl LoadProgress {
    /// What is happening, for the indicator.
    pub fn label(self) -> SharedString {
        match self {
            Self::ReadingCache => "Reading cache…".into(),
            Self::LoadingStack => "Loading the stack from GitHub…".into(),
            Self::CheckingCommits => "Checking GitHub for new commits…".into(),
            Self::FetchingFiles {
                fetched,
                total: Some(total),
            } => format!("Fetching files from GitHub… {fetched} of {total}").into(),
            Self::FetchingFiles {
                fetched,
                total: None,
            } => format!("Fetching files from GitHub… {fetched}").into(),
            Self::DownloadingDiff(download) => format!(
                "Downloading the diff… {:.1} MB",
                megabytes(download.received_bytes)
            )
            .into(),
            Self::BuildingDiffs { files } => format!("Building diffs of {files} files…").into(),
        }
    }

    /// How much is done, when it can be told: `0.0` to `1.0`.
    pub fn fraction(self) -> Option<f32> {
        match self {
            Self::FetchingFiles {
                fetched,
                total: Some(total),
            } if total > 0 => Some(ratio(u64::try_from(fetched).unwrap_or(u64::MAX), total)),
            // A compressed body reports its compressed length; past it, the size is unknown.
            Self::DownloadingDiff(DownloadProgress {
                received_bytes,
                total_bytes: Some(total),
            }) if total > 0 && received_bytes <= total => Some(ratio(received_bytes, total)),
            Self::ReadingCache
            | Self::LoadingStack
            | Self::CheckingCommits
            | Self::FetchingFiles { .. }
            | Self::DownloadingDiff(_)
            | Self::BuildingDiffs { .. } => None,
        }
    }

    /// Whether this step talks to GitHub (the indicator says so).
    pub fn is_network(self) -> bool {
        match self {
            Self::LoadingStack
            | Self::CheckingCommits
            | Self::FetchingFiles { .. }
            | Self::DownloadingDiff(_) => true,
            Self::ReadingCache | Self::BuildingDiffs { .. } => false,
        }
    }
}

/// `part / whole` as a fraction in `0.0..=1.0`, in thousandths.
fn ratio(part: u64, whole: u64) -> f32 {
    let steps = (part.min(whole) * FRACTION_STEPS / whole.max(1)).min(FRACTION_STEPS);
    let steps = u16::try_from(steps).unwrap_or(u16::MAX);
    f32::from(steps) / f32::from(u16::try_from(FRACTION_STEPS).unwrap_or(u16::MAX))
}

/// Bytes as megabytes, for display only.
fn megabytes(bytes: u64) -> f64 {
    f64::from(u32::try_from(bytes).unwrap_or(u32::MAX)) / BYTES_PER_MEGABYTE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A page count with a known total is a fraction; without one it is not.
    #[test]
    fn fetching_files_fraction() {
        let halfway = LoadProgress::FetchingFiles {
            fetched: 50,
            total: Some(100),
        };
        assert_eq!(halfway.fraction(), Some(0.5));
        let unknown = LoadProgress::FetchingFiles {
            fetched: 50,
            total: None,
        };
        assert_eq!(unknown.fraction(), None);
    }

    /// More bytes than the reported length (a compressed body) is no fraction.
    #[test]
    fn download_past_length_has_no_fraction() {
        let download = |received_bytes| {
            LoadProgress::DownloadingDiff(DownloadProgress {
                received_bytes,
                total_bytes: Some(1000),
            })
        };
        assert_eq!(download(250).fraction(), Some(0.25));
        assert_eq!(download(2000).fraction(), None);
    }
}
