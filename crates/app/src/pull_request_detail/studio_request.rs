use qd_file_tree::FileKind;
use qd_github::{PullRequest, PullRequestStack};
use qd_studio::{
    CodeAccess, FileListing, ReaderFamiliarity, RequestLayer, StudioRequest, StudioRequestFile,
};

use super::loaded_file::LoadedFile;
use super::review_target::ReviewTarget;

/// What the agent is told about the shown pull request at `target`, with
/// each loaded file's diff or why it is left out, for a combined range the
/// stack layers it spans (from `stack`), bottom first, and how well the
/// `reader` knows the area. The code access is set once the checkout ran.
pub(super) fn studio_request(
    pull_request: &PullRequest,
    target: &ReviewTarget,
    stack: Option<&PullRequestStack>,
    files: &[LoadedFile],
    reader: ReaderFamiliarity,
) -> StudioRequest {
    let layers = match (target, stack) {
        (ReviewTarget::Combined(range), Some(stack)) => stack
            .layers
            .get(range.first_index..=range.last_index)
            .unwrap_or_default()
            .iter()
            .map(|layer| RequestLayer {
                number: layer.number,
                title: layer.title.clone(),
                // The shown pull request's own; the others' are read with the run.
                description: (layer.number == pull_request.number)
                    .then(|| pull_request.body.clone())
                    .flatten(),
            })
            .collect(),
        _ => Vec::new(),
    };
    let subject = match target {
        ReviewTarget::Combined(range) => format!(
            "Stack layers #{} to #{} combined ({} layers), up to pull request #{}: {}",
            range.first_number,
            range.last_number,
            range.layer_count(),
            pull_request.number,
            pull_request.title
        ),
        ReviewTarget::Layer | ReviewTarget::AwaitingStack => {
            format!(
                "Pull request #{}: {}",
                pull_request.number, pull_request.title
            )
        }
    };
    StudioRequest {
        repository: pull_request.repo.full_name(),
        subject,
        author: pull_request.author.login.clone(),
        description: pull_request.body.clone().unwrap_or_default(),
        files: files
            .iter()
            .map(|loaded| StudioRequestFile {
                path: loaded.file.filename.clone(),
                previous_path: loaded.file.previous_filename.clone(),
                status: loaded.file.status.clone(),
                additions: loaded.file.additions,
                deletions: loaded.file.deletions,
                listing: listing_of(loaded),
            })
            .collect(),
        code: CodeAccess::DiffOnly("not prepared yet".into()),
        reader,
        layers,
    }
}

/// The file's diff for the agent, or why it is left out: generated and
/// unchanged moved files say little, and a file without a patch has none.
fn listing_of(loaded: &LoadedFile) -> FileListing {
    match (loaded.facets.kind, loaded.file.patch.is_some()) {
        (FileKind::Generated, _) => FileListing::LeftOut("generated".into()),
        (FileKind::Moved, _) => FileListing::LeftOut("moved without changes".into()),
        (FileKind::Code | FileKind::Tests, false) => {
            FileListing::LeftOut("no patch (binary, or too large for GitHub to show)".into())
        }
        (FileKind::Code | FileKind::Tests, true) => FileListing::Diff(loaded.diff.clone()),
    }
}
