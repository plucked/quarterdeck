use std::collections::HashMap;
use std::path::PathBuf;

use qd_claude::{ClaudeSession, GitBranch, GitRepository};

/// Prepared display and search text; full paths and UUIDs remain accessible
/// without making every row read like a registry record.
pub(super) struct SessionPresentation {
    pub name: String,
    pub subtitle: String,
    pub details: String,
    pub searchable: String,
    pub repository: Option<PathBuf>,
    folder: String,
}

impl SessionPresentation {
    /// Short folder labels keep the distinguishing worktree name at the start.
    /// Duplicate names get an ID suffix; all IDs are searchable.
    pub fn for_sessions(sessions: &[ClaudeSession]) -> Vec<Self> {
        // Add identity hints only where a name alone would be ambiguous.
        let mut counts = HashMap::new();
        for session in sessions {
            *counts.entry(&session.name).or_insert(0) += 1;
        }
        // Prepare labels once per refresh, not while scrolling or typing.
        sessions
            .iter()
            .map(|session| {
                let path = session.working_directory.display().to_string();
                let folder = session
                    .working_directory
                    .file_name()
                    .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
                let mut name = session.name.clone();
                if counts.get(&session.name).is_some_and(|count| *count > 1) {
                    name.push_str(" · ");
                    name.push_str(&session.id.as_str()[..8]);
                }
                Self {
                    name,
                    repository: None,
                    subtitle: folder.clone(),
                    folder,
                    details: format!("{}\n{path}\nSession {}", session.name, session.id.as_str()),
                    searchable: format!("{} {path} {}", session.name, session.id.as_str()),
                }
            })
            .collect()
    }

    /// Repository names and their main paths also find sessions in externally placed worktrees.
    pub fn set_repository(&mut self, repository: &GitRepository) {
        self.repository = Some(repository.path().to_owned());
        let path = repository.path().display().to_string();
        self.details.push_str("\nRepository: ");
        self.details.push_str(&path);
        self.searchable.push(' ');
        self.searchable.push_str(&path);
    }

    /// Adds a branch without moving the folder or session identity out of search.
    pub fn set_branch(&mut self, branch: &GitBranch) {
        self.subtitle = format!("{} · {}", branch.as_str(), self.folder);
        self.details.push_str("\nBranch: ");
        self.details.push_str(branch.as_str());
        self.searchable.push(' ');
        self.searchable.push_str(branch.as_str());
    }
}
