use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use qd_github::{FileTextLocation, GitHubError, PullRequestFile, RepoRef};

use crate::app_services::AppServices;

use super::search_content_result::SearchContentResult;

/// Reads complete texts at the loaded revision, using content-addressed cache entries first.
/// Deleted files use the base commit. The loop stops on cancellation or GitHub throttling.
pub(super) struct SearchContentJob {
    pub services: AppServices,
    pub repo: RepoRef,
    pub files: Vec<Arc<PullRequestFile>>,
    pub head: String,
    pub base: Option<String>,
}

impl SearchContentJob {
    /// Emits each full-text result in inventory order; cancellation prevents further requests.
    pub fn run(self, cancel: &AtomicBool, report: impl Fn(SearchContentResult)) {
        for file in &self.files {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            // A complete SHA is an immutable key. Abbreviations are resolved by revision/path.
            if let Some(sha) = file.sha.as_ref().filter(|sha| sha.len() == 40) {
                let cached = self.services.cache.lock().file_text(sha);
                if let Ok(Some(cached)) = cached {
                    report(SearchContentResult::Text {
                        path: file.filename.clone(),
                        text: cached.text.into(),
                    });
                    continue;
                }
            }
            let client = match &self.services.github {
                Ok(client) => client,
                Err(error) => {
                    report(SearchContentResult::Stopped {
                        message: error.clone(),
                        retry_after: None,
                    });
                    break;
                }
            };
            let Some(location) = search_location(file, &self.head, self.base.as_deref()) else {
                report(SearchContentResult::Failed(
                    "The base revision for a deleted file is unavailable.".into(),
                ));
                continue;
            };
            match client.file_text(&self.repo, &location) {
                Ok(text) => {
                    if let Err(error) = self.services.cache.lock().put_file_text(&text) {
                        eprintln!("cannot cache search text: {error:#}");
                    }
                    report(SearchContentResult::Text {
                        path: file.filename.clone(),
                        text: text.text.into(),
                    });
                }
                Err(GitHubError::NoFileText { .. }) => report(SearchContentResult::Skipped),
                Err(error) if error.is_throttled() => {
                    report(SearchContentResult::Stopped {
                        message: error.to_string(),
                        retry_after: Some(
                            error
                                .retry_after()
                                .unwrap_or(std::time::Duration::from_secs(60)),
                        ),
                    });
                    break;
                }
                Err(error) => report(SearchContentResult::Failed(error.to_string())),
            }
        }
    }
}

/// Selects the exact revision instead of accidentally searching a deleted file at HEAD.
fn search_location(
    file: &PullRequestFile,
    head: &str,
    base: Option<&str>,
) -> Option<FileTextLocation> {
    if let Some(sha) = file.sha.as_ref().filter(|sha| sha.len() == 40) {
        return Some(FileTextLocation::Blob(sha.clone()));
    }
    let commit = match file.status.as_str() {
        "removed" => base?,
        _ => head,
    };
    Some(FileTextLocation::Path {
        commit: commit.into(),
        path: file.filename.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deleted files without a complete blob SHA are searched in the base revision.
    #[test]
    fn removed_file_uses_base_and_current_file_uses_head() {
        let mut file = PullRequestFile {
            filename: "old.txt".into(),
            previous_filename: None,
            status: "removed".into(),
            additions: 0,
            deletions: 1,
            patch: None,
            sha: None,
        };
        assert!(
            matches!(search_location(&file,"head",Some("base")),Some(FileTextLocation::Path {commit,..}) if commit == "base")
        );
        assert!(search_location(&file, "head", None).is_none());
        file.status = "renamed".into();
        assert!(
            matches!(search_location(&file,"head",Some("base")),Some(FileTextLocation::Path {commit,..}) if commit == "head")
        );
    }
}
