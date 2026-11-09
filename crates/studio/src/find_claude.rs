use std::path::PathBuf;

/// Where the Claude Code CLI is looked for, after `PATH`: its own
/// installer's folder under the home folder, then Homebrew's. An app opened
/// from Finder gets launchd's `PATH`, which has none of them.
const CLAUDE_HOME_PATH: &str = ".local/bin/claude";
const CLAUDE_FIXED_PATHS: [&str; 2] = ["/opt/homebrew/bin/claude", "/usr/local/bin/claude"];

/// The `claude` executable to run, if one is installed.
pub fn find_claude() -> Option<PathBuf> {
    // `PATH` first, as a terminal would find it.
    let on_path = std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|folder| folder.join("claude"));
    // Then the installer's and Homebrew's folders.
    let in_home = dirs::home_dir().map(|home| home.join(CLAUDE_HOME_PATH));
    let fixed = CLAUDE_FIXED_PATHS.iter().map(PathBuf::from);
    on_path
        .chain(in_home)
        .chain(fixed)
        .find(|candidate| candidate.is_file())
}
