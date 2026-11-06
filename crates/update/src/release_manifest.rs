use serde::Deserialize;

use crate::{AppVersion, UpdateError};

/// Where release downloads must come from: this repository's releases.
const RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/plucked/quarterdeck/releases/download/";
/// The length of a SHA-256 digest in hex.
const SHA256_HEX_LENGTH: usize = 64;

/// The newest release as `latest.json`, an asset of every GitHub release,
/// describes it: its version, notes and the zipped app to download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseManifest {
    pub version: AppVersion,
    /// The release notes, GitHub-flavored Markdown.
    pub notes: String,
    /// The zipped `Quarterdeck.app`, on this repository's releases.
    pub zip_url: String,
    /// The zip's SHA-256, lowercase hex.
    pub zip_sha256: String,
    /// The zip's size in bytes; a larger download is refused.
    pub zip_size: u64,
}

/// `latest.json` as written by `packaging/macos/release.sh`.
#[derive(Deserialize)]
struct ManifestJson {
    version: String,
    #[serde(default)]
    notes: String,
    zip_url: String,
    zip_sha256: String,
    zip_size: u64,
}

impl ReleaseManifest {
    /// Reads and checks a manifest: the version parses, the download comes
    /// from this repository's releases, and the digest is SHA-256 hex.
    pub fn parse(json: &str) -> Result<Self, UpdateError> {
        let manifest: ManifestJson = serde_json::from_str(json)?;
        if !manifest.zip_url.starts_with(RELEASE_DOWNLOAD_PREFIX) {
            return Err(UpdateError::InvalidManifest(
                "the download is not on Quarterdeck's GitHub releases",
            ));
        }
        let digest = manifest.zip_sha256.to_ascii_lowercase();
        if digest.len() != SHA256_HEX_LENGTH || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(UpdateError::InvalidManifest("the checksum is not SHA-256"));
        }
        Ok(Self {
            version: AppVersion::parse(&manifest.version)?,
            notes: manifest.notes,
            zip_url: manifest.zip_url,
            zip_sha256: digest,
            zip_size: manifest.zip_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_json(zip_url: &str, digest: &str) -> String {
        serde_json::json!({
            "version": "0.2.0",
            "notes": "- New updater",
            "zip_url": zip_url,
            "zip_sha256": digest,
            "zip_size": 1234,
        })
        .to_string()
    }

    /// A manifest from this repository's releases reads whole.
    #[test]
    fn reads_a_release_manifest() {
        let digest = "AB".repeat(32);
        let manifest = ReleaseManifest::parse(&manifest_json(
            "https://github.com/plucked/quarterdeck/releases/download/v0.2.0/Quarterdeck-0.2.0.zip",
            &digest,
        ))
        .unwrap();
        assert_eq!(manifest.version.to_string(), "0.2.0");
        assert_eq!(manifest.zip_sha256, "ab".repeat(32));
        assert_eq!(manifest.zip_size, 1234);
        assert_eq!(manifest.notes, "- New updater");
    }

    /// A download from anywhere else is refused.
    #[test]
    fn refuses_foreign_downloads() {
        let json = manifest_json("https://example.com/Quarterdeck.zip", &"ab".repeat(32));
        assert!(matches!(
            ReleaseManifest::parse(&json),
            Err(UpdateError::InvalidManifest(_))
        ));
    }

    /// A digest that is not SHA-256 hex is refused.
    #[test]
    fn refuses_bad_digests() {
        let url = "https://github.com/plucked/quarterdeck/releases/download/v0.2.0/q.zip";
        for digest in ["abc".to_owned(), "zz".repeat(32)] {
            assert!(ReleaseManifest::parse(&manifest_json(url, &digest)).is_err());
        }
    }
}
