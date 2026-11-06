use std::path::PathBuf;

use crate::AppVersion;

/// A downloaded release, unpacked beside the running bundle and verified,
/// ready for [`crate::RunningBundle::install`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedUpdate {
    pub version: AppVersion,
    /// The unpacked `Quarterdeck.app`.
    pub(crate) app: PathBuf,
}
