use crate::github_client::PAGE_SIZE;
use crate::unified_diff::files_of_unified_diff;
use crate::{DownloadProgress, GitHubClient, GitHubError, PullRequestFile, RepoRef};

/// A comparison as one `git diff` text: every file, unlike the JSON
/// comparison, which stops at 300.
const DIFF_MEDIA_TYPE: &str = "application/vnd.github.diff";
/// GitHub lists at most 3000 changed files per pull request: 30 pages of 100.
const MAX_FILE_PAGES: usize = 30;
/// The most changed files `pull_request_files` can return; a larger pull
/// request's files come whole from `compare_files` instead.
pub const MAX_LISTED_FILES: u64 = 3000;

impl GitHubClient {
    /// Every file a pull request changes, with its patch, following the
    /// pages (100 files each, at most 3000 files in all). Reports the count
    /// fetched so far after each page.
    pub fn pull_request_files(
        &self,
        repo: &RepoRef,
        number: u64,
        on_page: &mut dyn FnMut(usize),
    ) -> Result<Vec<PullRequestFile>, GitHubError> {
        // GitHub stops listing at its cap, so a capped read is all there is.
        let mut files = Vec::new();
        self.linked_pages(
            &format!("{}/pulls/{number}/files", repo.api_url()),
            &[("per_page", PAGE_SIZE)],
            MAX_FILE_PAGES,
            &mut |page: Vec<PullRequestFile>| {
                files.extend(page);
                on_page(files.len());
                Ok(())
            },
        )?;
        Ok(files)
    }

    /// Every file changed from `base_sha` to `head_sha` (from their merge
    /// base, as a pull request compares), with patches: one request for the
    /// whole `git diff`, reporting the bytes received as they arrive.
    pub fn compare_files(
        &self,
        repo: &RepoRef,
        base_sha: &str,
        head_sha: &str,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<Vec<PullRequestFile>, GitHubError> {
        let url = format!("{}/compare/{base_sha}...{head_sha}", repo.api_url());
        let response = self.get(&url, &[], None, DIFF_MEDIA_TYPE, on_progress)?;
        Ok(files_of_unified_diff(&response.body))
    }
}
