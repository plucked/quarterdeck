use std::path::Path;
use std::process::Command;

use crate::UpdateError;

/// What a Quarterdeck release's signature must satisfy: the app's bundle
/// identifier, signed with a Developer ID certificate issued by Apple to
/// the developer's team (the designated requirement of a Developer ID app).
const RELEASE_REQUIREMENT: &str = concat!(
    "=identifier \"de.plucked.quarterdeck\" and anchor apple generic",
    " and certificate 1[field.1.2.840.113635.100.6.2.6]",
    " and certificate leaf[field.1.2.840.113635.100.6.1.13]",
    " and certificate leaf[subject.OU] = \"97ZZEC8R43\""
);

/// Checks that the bundle at `app` is an intact Quarterdeck release: every
/// file matches its signature, the signer is the developer's Developer ID,
/// and Gatekeeper accepts it (it is notarized).
pub fn verify_release_signature(app: &Path) -> Result<(), UpdateError> {
    // The signature covers the whole bundle and names the right signer.
    run_check(
        "codesign",
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(format!("-R{RELEASE_REQUIREMENT}"))
            .arg(app),
    )?;
    // Gatekeeper would open it: notarized, not revoked.
    run_check(
        "spctl",
        Command::new("/usr/sbin/spctl")
            .args(["--assess", "--type", "execute"])
            .arg(app),
    )
}

/// Whether the bundle at `app` is signed as a release (Developer ID of the
/// developer's team); a development build is signed ad hoc and is not.
pub fn is_release_signed(app: &Path) -> bool {
    Command::new("/usr/bin/codesign")
        .args(["--verify", "--strict"])
        .arg(format!("-R{RELEASE_REQUIREMENT}"))
        .arg(app)
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Runs `command`; a failure carries what it printed.
fn run_check(tool: &'static str, command: &mut Command) -> Result<(), UpdateError> {
    let output = command.output().map_err(|error| UpdateError::Tool {
        tool,
        message: error.to_string(),
    })?;
    if output.status.success() {
        Ok(())
    } else {
        Err(UpdateError::Signature(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}
