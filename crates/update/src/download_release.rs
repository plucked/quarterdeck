use std::fmt::Write as _;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{ReleaseManifest, UpdateError, UpdateFeed};

/// How much of the download is read and hashed at a time.
const READ_CHUNK_BYTES: usize = 256 * 1024;

impl UpdateFeed {
    /// Downloads the release's zip into `directory` (created if needed),
    /// hashing it while it streams to disk, and returns the file once its
    /// size and SHA-256 match the manifest; a mismatch removes it.
    pub fn download_release(
        &self,
        release: &ReleaseManifest,
        directory: &Path,
    ) -> Result<PathBuf, UpdateError> {
        // Start the request; GitHub redirects to its asset storage, which ureq follows.
        let mut response = self.agent().get(&release.zip_url).call()?;
        let status = response.status();
        if !status.is_success() {
            return Err(UpdateError::Status {
                status: status.as_u16(),
            });
        }

        // Stream the body to disk, hashing it and refusing more than announced.
        std::fs::create_dir_all(directory)
            .map_err(|error| UpdateError::file("cannot create", directory, error))?;
        let path = directory.join(format!("Quarterdeck-{}.zip", release.version));
        let written = write_hashed(
            &mut response.body_mut().as_reader(),
            &path,
            release.zip_size,
        );

        // Keep the file only when it is exactly what the manifest describes.
        let verdict = written.and_then(|written| {
            match (
                written.size == release.zip_size,
                written.sha256 == release.zip_sha256,
            ) {
                (true, true) => Ok(()),
                (false, _) => Err(UpdateError::TooLarge),
                (true, false) => Err(UpdateError::ChecksumMismatch),
            }
        });
        match verdict {
            Ok(()) => Ok(path),
            Err(error) => {
                std::fs::remove_file(&path).ok();
                Err(error)
            }
        }
    }
}

/// What [`write_hashed`] wrote.
struct WrittenFile {
    size: u64,
    /// Lowercase hex.
    sha256: String,
}

/// Copies `reader` to a new file at `path`, hashing it on the way; stops
/// with [`UpdateError::TooLarge`] past `max_size` bytes.
fn write_hashed(
    reader: &mut dyn Read,
    path: &Path,
    max_size: u64,
) -> Result<WrittenFile, UpdateError> {
    let mut file =
        File::create(path).map_err(|error| UpdateError::file("cannot create", path, error))?;
    let mut hasher = Sha256::new();
    let mut size: u64 = 0;
    let mut chunk = vec![0; READ_CHUNK_BYTES];
    loop {
        let read = reader.read(&mut chunk).map_err(ureq::Error::from)?;
        if read == 0 {
            break;
        }
        size += u64::try_from(read).unwrap_or(u64::MAX);
        if size > max_size {
            return Err(UpdateError::TooLarge);
        }
        hasher.update(&chunk[..read]);
        file.write_all(&chunk[..read])
            .map_err(|error| UpdateError::file("cannot write", path, error))?;
    }
    file.sync_all()
        .map_err(|error| UpdateError::file("cannot write", path, error))?;
    let sha256 = hasher
        .finalize()
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").ok();
            hex
        });
    Ok(WrittenFile { size, sha256 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("qd-update-{}-{name}", std::process::id()))
    }

    /// The hash is the SHA-256 of what was written.
    #[test]
    fn hashes_what_it_writes() {
        let path = temp_file("hash");
        let written = write_hashed(&mut "abc".as_bytes(), &path, 10).unwrap();
        assert_eq!(written.size, 3);
        assert_eq!(
            written.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"abc");
        std::fs::remove_file(path).ok();
    }

    /// A body larger than announced stops the download.
    #[test]
    fn stops_past_the_announced_size() {
        let path = temp_file("large");
        let written = write_hashed(&mut "abcdef".as_bytes(), &path, 4);
        assert!(matches!(written, Err(UpdateError::TooLarge)));
        std::fs::remove_file(path).ok();
    }
}
