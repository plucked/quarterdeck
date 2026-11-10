use qd_cache::{
    BriefIndex, NoteDestination, ReviewNote, ReviewScope, ReviewSummary, StoredBrief, ViewedFile,
};
use qd_studio::{BriefRecord, Studio};

use super::SharedUiStateStore;

/// The review notes of `scope` (both destinations) and, for a pull request,
/// the GitHub review's summary, read through the shared store on the
/// background executor; unreadable rows count as none.
pub fn read_review_notes(
    store: &SharedUiStateStore,
    scope: &ReviewScope,
) -> (Vec<ReviewNote>, Option<ReviewSummary>) {
    let store = store.lock();
    let mut notes = Vec::new();
    for destination in [NoteDestination::Local, NoteDestination::GitHub] {
        match store.review_notes(scope, destination) {
            Ok(read) => notes.extend(read),
            Err(error) => eprintln!("ui state: ignoring review notes: {error:#}"),
        }
    }
    let summary = match scope {
        ReviewScope::PullRequest(pull_request_id) => store
            .review_summary(*pull_request_id)
            .unwrap_or_else(|error| {
                eprintln!("ui state: ignoring the review summary: {error:#}");
                None
            }),
        ReviewScope::CombinedLayers { .. } => None,
    };
    (notes, summary)
}

/// The stored brief of `scope`, read through the shared store on the
/// background executor; an unreadable brief counts as none.
pub fn read_brief(store: &SharedUiStateStore, scope: &ReviewScope) -> Option<StoredBrief> {
    store.lock().brief(scope).unwrap_or_else(|error| {
        eprintln!("ui state: ignoring a stored brief: {error:#}");
        None
    })
}

/// Stores the brief `studio` with its `record` as the brief of `scope`,
/// through the shared store on the background executor; a failure is
/// reported on stderr (the brief still shows for the session).
pub fn write_brief(
    store: &SharedUiStateStore,
    scope: &ReviewScope,
    studio: &Studio,
    record: &BriefRecord,
) {
    if let Err(error) = store.lock().put_brief(scope, studio, record) {
        eprintln!("ui state: cannot store a brief: {error:#}");
    }
}

/// Which pull requests have a stored brief, read through the shared store
/// on the background executor; unreadable means none.
pub fn read_brief_index(store: &SharedUiStateStore) -> BriefIndex {
    store.lock().brief_index().unwrap_or_else(|error| {
        eprintln!("ui state: ignoring the stored briefs: {error:#}");
        BriefIndex::default()
    })
}

/// The viewed marks of a pull request, read through the shared store on the
/// background executor; an unreadable table counts as no marks.
pub fn read_viewed_files(store: &SharedUiStateStore, scope: &ReviewScope) -> Vec<ViewedFile> {
    store.lock().viewed_files(scope).unwrap_or_else(|error| {
        eprintln!("ui state: ignoring viewed marks: {error:#}");
        Vec::new()
    })
}
