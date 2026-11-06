use std::path::PathBuf;

/// Every way checking for, downloading or installing an update can fail.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// The request did not complete (DNS, TLS, connection, body read).
    #[error("network error: {0}")]
    Http(#[from] ureq::Error),
    /// The release feed answered with a non-success status.
    #[error("the release feed returned {status}")]
    Status { status: u16 },
    /// The release manifest is not the expected JSON.
    #[error("the release manifest cannot be read: {0}")]
    Manifest(#[from] serde_json::Error),
    /// The manifest holds something it must not (a foreign download URL, a bad digest).
    #[error("the release manifest is invalid: {0}")]
    InvalidManifest(&'static str),
    /// A version that is not `major.minor.patch`.
    #[error("invalid version {0:?}")]
    InvalidVersion(String),
    /// The download is larger than the manifest said.
    #[error("the download is larger than announced")]
    TooLarge,
    /// The download's SHA-256 differs from the manifest's.
    #[error("the download is damaged (its checksum does not match)")]
    ChecksumMismatch,
    /// The downloaded app is not signed by the app's developer, or
    /// Gatekeeper refuses it.
    #[error("the downloaded app's signature is not valid: {0}")]
    Signature(String),
    /// The downloaded app is not the version the manifest announced.
    #[error("the downloaded app is version {found}, not {expected}")]
    WrongVersion { expected: String, found: String },
    /// A file operation failed.
    #[error("{action} {}: {source}", path.display())]
    File {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    /// A system tool (`ditto`, `codesign`, `plutil`) could not run or failed.
    #[error("{tool} failed: {message}")]
    Tool { tool: &'static str, message: String },
}

impl UpdateError {
    /// A file error: `action` completes "… {path}" (e.g. "cannot create").
    pub(crate) fn file(
        action: &'static str,
        path: &std::path::Path,
        source: std::io::Error,
    ) -> Self {
        Self::File {
            action,
            path: path.to_owned(),
            source,
        }
    }
}
