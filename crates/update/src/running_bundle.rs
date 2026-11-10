use std::path::{Path, PathBuf};
use std::process::Command;

use crate::code_signature::{is_release_signed, verify_release_signature};
use crate::run_system_tool::{ToolRefusal, run_system_tool};
use crate::{AppVersion, ReleaseManifest, StagedUpdate, UpdateBlocker, UpdateError};

/// The folder macOS runs quarantined apps from when they were not moved
/// (App Translocation): a read-only, randomized copy.
const TRANSLOCATION_MARKER: &str = "/AppTranslocation/";
/// The app inside a release zip.
const ZIPPED_APP_NAME: &str = "Quarterdeck.app";
/// Where the running bundle goes while the new one takes its place.
const PREVIOUS_APP_NAME: &str = "previous.app";

/// The app bundle the running process was started from, when it can be
/// replaced by an update.
#[derive(Debug, Clone)]
pub struct RunningBundle {
    /// The `.app` folder, e.g. `/Applications/Quarterdeck.app`.
    path: PathBuf,
}

impl RunningBundle {
    /// Finds the running bundle and checks that it can update itself: it is
    /// a bundle, not translocated, signed as a release and in a folder this
    /// user can write. Runs `codesign`, so call it off the main thread.
    pub fn locate() -> Result<Self, UpdateBlocker> {
        let executable = std::env::current_exe()
            .and_then(std::fs::canonicalize)
            .map_err(|_| UpdateBlocker::NotABundle)?;
        let path = bundle_of(&executable).ok_or(UpdateBlocker::NotABundle)?;
        if path.to_string_lossy().contains(TRANSLOCATION_MARKER) {
            return Err(UpdateBlocker::Translocated);
        }
        if !is_release_signed(&path) {
            return Err(UpdateBlocker::DevelopmentBuild);
        }
        let bundle = Self { path };
        if !bundle.can_write_beside() {
            return Err(UpdateBlocker::ReadOnlyLocation);
        }
        Ok(bundle)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Unpacks the downloaded `zip` next to the running bundle (on the same
    /// volume, so installing is two renames), then checks that it holds a
    /// Quarterdeck release of `release`'s version with a valid signature.
    pub fn stage_release(
        &self,
        zip: &Path,
        release: &ReleaseManifest,
    ) -> Result<StagedUpdate, UpdateError> {
        // A fresh staging folder: whatever an earlier attempt left goes.
        let staging = self.staging_directory();
        remove_directory(&staging)?;
        std::fs::create_dir_all(&staging)
            .map_err(|error| UpdateError::file("cannot create", &staging, error))?;

        // Unpack with ditto, which keeps the bundle's signature and attributes intact.
        run_system_tool(
            "ditto",
            Command::new("/usr/bin/ditto")
                .args(["-x", "-k"])
                .arg(zip)
                .arg(&staging),
            ToolRefusal::Failed,
        )?;
        let app = staging.join(ZIPPED_APP_NAME);

        // Only a release signed by the developer and of the announced version is installed.
        verify_release_signature(&app)?;
        let found = bundle_version(&app)?;
        if found != release.version {
            return Err(UpdateError::WrongVersion {
                expected: release.version.to_string(),
                found: found.to_string(),
            });
        }
        Ok(StagedUpdate {
            version: found,
            app,
        })
    }

    /// Puts the staged app in place of the running bundle: the running one
    /// moves into the staging folder and the new one takes its path. The
    /// process keeps running from the moved files; the next launch opens
    /// the new version. A failed second step moves the old bundle back.
    pub fn install(&self, staged: &StagedUpdate) -> Result<(), UpdateError> {
        // Move the running bundle aside, then the new one into its place.
        let previous = self.staging_directory().join(PREVIOUS_APP_NAME);
        remove_directory(&previous)?;
        std::fs::rename(&self.path, &previous)
            .map_err(|error| UpdateError::file("cannot move", &self.path, error))?;
        if let Err(error) = std::fs::rename(&staged.app, &self.path) {
            std::fs::rename(&previous, &self.path).ok();
            return Err(UpdateError::file("cannot move", &staged.app, error));
        }

        // The old bundle is no longer needed; a failure leaves it for the next launch to remove.
        self.remove_leftovers();
        Ok(())
    }

    /// Removes the staging folder an earlier update left (the bundle it
    /// replaced, or an update never installed).
    pub fn remove_leftovers(&self) {
        if let Err(error) = remove_directory(&self.staging_directory()) {
            eprintln!("cannot remove the update's staging folder: {error}");
        }
    }

    /// `.Quarterdeck.app-update` beside the bundle: hidden, on the same volume.
    fn staging_directory(&self) -> PathBuf {
        let name = self
            .path
            .file_name()
            .map_or_else(|| ZIPPED_APP_NAME.into(), |name| name.to_string_lossy());
        self.parent().join(format!(".{name}-update"))
    }

    fn parent(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("/"))
    }

    /// Whether this user can create (and so rename) entries beside the bundle.
    fn can_write_beside(&self) -> bool {
        let probe = self
            .parent()
            .join(format!(".quarterdeck-write-check-{}", std::process::id()));
        let can_write = std::fs::create_dir(&probe).is_ok();
        std::fs::remove_dir(&probe).ok();
        can_write
    }
}

/// The `.app` folder holding `executable` at `Contents/MacOS/`, if any.
pub(crate) fn bundle_of(executable: &Path) -> Option<PathBuf> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let is_bundle = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle.extension()? == "app";
    is_bundle.then(|| bundle.to_owned())
}

/// The bundle's `CFBundleShortVersionString`.
fn bundle_version(app: &Path) -> Result<AppVersion, UpdateError> {
    let info = app.join("Contents/Info.plist");
    let output = run_system_tool(
        "plutil",
        Command::new("/usr/bin/plutil")
            .args(["-extract", "CFBundleShortVersionString", "raw", "-o", "-"])
            .arg(&info),
        ToolRefusal::Failed,
    )?;
    AppVersion::parse(&output)
}

/// Removes `directory` and everything in it; nothing to do when it is missing.
fn remove_directory(directory: &Path) -> Result<(), UpdateError> {
    match std::fs::remove_dir_all(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(UpdateError::file("cannot remove", directory, error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An executable at `X.app/Contents/MacOS/` belongs to `X.app`.
    #[test]
    fn finds_the_bundle_of_an_executable() {
        assert_eq!(
            bundle_of(Path::new(
                "/Applications/Quarterdeck.app/Contents/MacOS/Quarterdeck"
            )),
            Some(PathBuf::from("/Applications/Quarterdeck.app"))
        );
    }

    /// A bare executable (`cargo run`) has no bundle.
    #[test]
    fn a_bare_executable_has_no_bundle() {
        assert_eq!(bundle_of(Path::new("/repo/target/debug/quarterdeck")), None);
        assert_eq!(bundle_of(Path::new("/x/Contents/MacOS/Quarterdeck")), None);
    }

    /// The staging folder is hidden beside the bundle.
    #[test]
    fn stages_beside_the_bundle() {
        let bundle = RunningBundle {
            path: PathBuf::from("/Applications/Quarterdeck.app"),
        };
        assert_eq!(
            bundle.staging_directory(),
            PathBuf::from("/Applications/.Quarterdeck.app-update")
        );
    }

    /// Installing swaps the bundles and removes the staging folder.
    #[test]
    fn install_swaps_the_bundles() {
        // Set up: a "running" bundle and a staged one beside it.
        let root = std::env::temp_dir().join(format!("qd-update-install-{}", std::process::id()));
        std::fs::remove_dir_all(&root).ok();
        let bundle = RunningBundle {
            path: root.join("Quarterdeck.app"),
        };
        std::fs::create_dir_all(bundle.path.join("Contents")).unwrap();
        std::fs::write(bundle.path.join("Contents/version"), "old").unwrap();
        let staged_app = bundle.staging_directory().join(ZIPPED_APP_NAME);
        std::fs::create_dir_all(staged_app.join("Contents")).unwrap();
        std::fs::write(staged_app.join("Contents/version"), "new").unwrap();
        let staged = StagedUpdate {
            version: AppVersion::current(),
            app: staged_app,
        };

        // Act and expect: the new bundle is in place, nothing else is left.
        bundle.install(&staged).unwrap();
        assert_eq!(
            std::fs::read_to_string(bundle.path.join("Contents/version")).unwrap(),
            "new"
        );
        assert!(!bundle.staging_directory().exists());
        std::fs::remove_dir_all(root).ok();
    }
}
