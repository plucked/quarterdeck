use std::path::Path;
use std::time::Duration;

use crate::command_output::command_output;
use crate::git_command::git_command;

const BRANCH_TIMEOUT: Duration = Duration::from_secs(2);

/// A branch name reported by Git for a session's current checkout.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitBranch(String);

impl GitBranch {
    /// Reads HEAD without modifying the checkout. Git resolves linked worktrees
    /// and nested directories itself; detached HEAD and non-repositories return none.
    pub fn read(directory: &Path) -> Option<Self> {
        let mut command = git_command(directory);
        command.args(["symbolic-ref", "--quiet", "--short", "HEAD"]);
        let output = command_output(&mut command, BRANCH_TIMEOUT).ok()?;
        let name = std::str::from_utf8(&output).ok()?.trim();
        if name.is_empty() {
            return None;
        }
        Some(Self(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GitRepository;

    /// Runs local fixture setup with no user Git configuration, hooks or signing.
    fn git(directory: &Path, arguments: &[&str]) {
        // Keep fixture setup inside its own repository regardless of the caller.
        let mut command = git_command(directory);
        command
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgSign=false",
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
            ])
            .args(arguments)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null");
        command
            .env_remove("GIT_CONFIG_COUNT")
            .env_remove("GIT_CONFIG_PARAMETERS")
            .env_remove("GIT_TEMPLATE_DIR");
        command_output(&mut command, Duration::from_secs(5)).unwrap();
    }

    /// Linked and detached worktrees share a repository while branches and separate clones stay distinct.
    #[test]
    fn reads_repository_and_worktree_branches() {
        let directory = tempfile::tempdir().unwrap();
        let repository = directory.path().join("repository with space\nand newline");
        let worktree = directory.path().join("worktree");
        std::fs::create_dir(&repository).unwrap();
        git(&repository, &["init", "-b", "main"]);
        git(&repository, &["commit", "--allow-empty", "-m", "Fixture"]);
        git(
            &repository,
            &[
                "worktree",
                "add",
                "-b",
                "feature/review",
                worktree.to_str().unwrap(),
            ],
        );
        let nested = worktree.join("nested");
        std::fs::create_dir(&nested).unwrap();

        assert_eq!(GitBranch::read(&repository).unwrap().as_str(), "main");
        assert_eq!(GitBranch::read(&nested).unwrap().as_str(), "feature/review");
        let main_repository = GitRepository::read(&repository).unwrap();
        assert_eq!(main_repository.path(), repository.canonicalize().unwrap());
        assert_eq!(GitRepository::read(&nested).unwrap(), main_repository);
        git(&worktree, &["checkout", "--detach"]);
        assert_eq!(GitBranch::read(&worktree), None);
        assert_eq!(GitRepository::read(&worktree).unwrap(), main_repository);
        assert_eq!(GitRepository::read(directory.path()), None);
        // Independent clones with the same folder name must remain separate groups.
        let other_repository = directory
            .path()
            .join("other")
            .join(repository.file_name().unwrap());
        std::fs::create_dir_all(&other_repository).unwrap();
        git(&other_repository, &["init", "-b", "main"]);
        assert_ne!(
            GitRepository::read(&other_repository).unwrap(),
            main_repository
        );
        assert_eq!(GitBranch::read(directory.path()), None);
    }
}
