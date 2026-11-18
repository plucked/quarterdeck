use std::collections::HashMap;

use qd_cache::{
    BriefIndex, NoteDestination, ReviewNote, ReviewScope, ReviewSummary, StoredBrief, ViewedFile,
};

use super::SharedUiStateStore;
use super::pending_ui_state::ViewedChange;

/// The review notes of `scope` (both destinations) and, for a pull request,
/// the GitHub review's summary, read through the shared store on the
/// background executor; unreadable rows count as none.
pub fn read_review_notes(
    store: &SharedUiStateStore,
    scope: &ReviewScope,
) -> (Vec<ReviewNote>, Option<ReviewSummary>) {
    let shared = store;
    let store = shared.lock();
    let mut notes = Vec::new();
    for destination in [NoteDestination::Local, NoteDestination::GitHub] {
        match store.review_notes(scope, destination) {
            Ok(read) => notes.extend(read),
            Err(error) => eprintln!("ui state: ignoring review notes: {error:#}"),
        }
    }
    let mut summary = match scope {
        ReviewScope::PullRequest(pull_request_id) => store
            .review_summary(*pull_request_id)
            .unwrap_or_else(|error| {
                eprintln!("ui state: ignoring the review summary: {error:#}");
                None
            }),
        ReviewScope::CombinedLayers { .. } => None,
    };
    for change in shared.review_writes().iter() {
        change.overlay(scope, &mut notes, &mut summary);
    }
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

/// Which pull requests have a stored brief, read through the shared store
/// on the background executor; unreadable means none.
pub fn read_brief_index(store: &SharedUiStateStore) -> BriefIndex {
    store.lock().brief_index().unwrap_or_else(|error| {
        eprintln!("ui state: ignoring the stored briefs: {error:#}");
        BriefIndex::default()
    })
}

/// Viewed marks for this scope, including edits waiting for their batched write.
pub fn read_viewed_files(shared: &SharedUiStateStore, scope: &ReviewScope) -> Vec<ViewedFile> {
    // Keep the writer from moving an edit into SQLite between the two reads.
    let store = shared.lock();
    let mut marks = store.viewed_files(scope).unwrap_or_else(|error| {
        eprintln!("ui state: ignoring viewed marks: {error:#}");
        Vec::new()
    });
    let mut edits = HashMap::new();
    for change in shared.viewed_changes().iter() {
        match change {
            ViewedChange::Mark {
                scope: edited,
                file,
            } if edited == scope => {
                edits.insert(file.filename.clone(), Some(file.fingerprint));
            }
            ViewedChange::Unmark {
                scope: edited,
                filename,
            } if edited == scope => {
                edits.insert(filename.clone(), None);
            }
            _ => {}
        }
    }
    drop(store);

    // The last queued edit for each filename replaces its saved row.
    if !edits.is_empty() {
        marks.retain(|mark| !edits.contains_key(&mark.filename));
        marks.extend(edits.into_iter().filter_map(|(filename, fingerprint)| {
            Some(ViewedFile {
                filename,
                fingerprint: fingerprint?,
            })
        }));
        marks.sort_unstable_by(|left, right| left.filename.cmp(&right.filename));
    }
    marks
}
