use qd_diff::{DiffOptions, FileDiff};
use qd_studio::AnchorTargets;

use crate::brief_view::BriefDiffs;

/// The sample files: path and GitHub-style patch (hunks only), from the
/// commit that made a used-up rate limit hold the refresh schedule.
const SAMPLE_PATCHES: [(&str, &str); 2] = [
    (
        "crates/github/src/github_error.rs",
        include_str!("samples/github_error.patch"),
    ),
    (
        "crates/app/src/review_queue/refresh_schedule.rs",
        include_str!("samples/refresh_schedule.patch"),
    ),
];

/// One sample file and its diff.
struct SampleFile {
    path: &'static str,
    diff: FileDiff,
}

/// The diffs the gallery's sample brief quotes, built from patches
/// compiled into the app.
pub struct SampleDiffs {
    files: Vec<SampleFile>,
}

impl SampleDiffs {
    /// Builds every sample file's diff with the default options.
    pub fn build() -> Self {
        let files = SAMPLE_PATCHES
            .iter()
            .map(|(path, patch)| SampleFile {
                path,
                diff: FileDiff::build(patch, DiffOptions::default()),
            })
            .collect();
        Self { files }
    }

    /// The lines the sample diffs show, which the sample's anchors are checked against.
    pub fn anchor_targets(&self) -> AnchorTargets {
        let mut targets = AnchorTargets::default();
        for file in &self.files {
            targets.add_file(file.path, None, &file.diff);
        }
        targets
    }
}

impl BriefDiffs for SampleDiffs {
    fn diff_of(&self, path: &str) -> Option<&FileDiff> {
        self.files
            .iter()
            .find(|file| file.path == path)
            .map(|file| &file.diff)
    }
}
