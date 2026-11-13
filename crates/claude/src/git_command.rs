use std::path::Path;
use std::process::Command;

/// A session's directory, not the app's launching repository, chooses the checkout.
const REPOSITORY_VARIABLES: [&str; 10] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_COMMON_DIR",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_NAMESPACE",
    "GIT_CEILING_DIRECTORIES",
    "GIT_DISCOVERY_ACROSS_FILESYSTEM",
    "GIT_SHALLOW_FILE",
];

/// Prepares a local Git read with no optional locks or inherited repository override.
pub(crate) fn git_command(directory: &Path) -> Command {
    let mut command = Command::new("git");
    command.args(["--no-optional-locks", "-C"]).arg(directory);
    for variable in REPOSITORY_VARIABLES {
        command.env_remove(variable);
    }
    command
}
