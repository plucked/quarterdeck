use std::collections::HashMap;
use std::sync::Arc;

use gpui::SharedString;
use qd_cache::{NoteAnchor, NoteDestination, NoteId, ReviewNote, ReviewSummary};
use qd_github::{LineSide, PullRequestReviewState};
use qd_markdown::MarkdownDocument;

/// One card of comments in the diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentCard {
    /// A GitHub conversation (an index into the review state's threads),
    /// with the viewer's draft replies to it.
    Thread { thread: usize, replies: Vec<NoteId> },
    /// One of the user's notes: a local note, or a draft of a new GitHub
    /// conversation.
    Note(NoteId),
}

/// A card with where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedCard {
    pub card: CommentCard,
    pub path: String,
    /// The line it sits under; `None` for an outdated GitHub conversation,
    /// shown at the top of its file.
    pub anchor: Option<(LineSide, u32)>,
    /// The first line of a comment on several lines.
    pub start_line: Option<u32>,
}

/// The review comments of the shown files: GitHub's conversations (a single
/// pull request only), the user's notes (local ones and GitHub drafts) and
/// the GitHub review's summary, with every body parsed once for display.
#[derive(Default)]
pub struct ReviewComments {
    pub github: Option<PullRequestReviewState>,
    /// Why GitHub's conversations could not be read.
    pub github_error: Option<SharedString>,
    pub is_loading_github: bool,
    /// The user's notes of the shown scope, both destinations.
    pub notes: Vec<ReviewNote>,
    pub summary: ReviewSummary,
    /// Parsed bodies by comment key (a GitHub node id, or `note-<id>`), with
    /// the text they were parsed from.
    documents: HashMap<String, (String, Arc<MarkdownDocument>)>,
}

impl ReviewComments {
    /// The cards to show: GitHub's conversations and drafts (with
    /// `include_github`, on a single pull request) and the local notes.
    pub fn cards(&self, include_github: bool) -> Vec<PlacedCard> {
        let mut cards = Vec::new();

        // GitHub's conversations, each with the drafts replying to it.
        if include_github && let Some(state) = &self.github {
            for (index, thread) in state.threads.iter().enumerate() {
                let replies = self
                    .notes
                    .iter()
                    .filter(|note| {
                        note.destination == NoteDestination::GitHub
                            && matches!(&note.anchor, NoteAnchor::Reply { thread_id } if *thread_id == thread.id)
                    })
                    .map(|note| note.id)
                    .collect();
                cards.push(PlacedCard {
                    card: CommentCard::Thread {
                        thread: index,
                        replies,
                    },
                    path: thread.path.clone(),
                    anchor: thread.line.map(|line| (thread.side, line)),
                    start_line: thread.start_line,
                });
            }
        }

        // The user's notes on lines: GitHub drafts only where GitHub conversations show.
        for note in self
            .notes
            .iter()
            .filter(|note| include_github || note.destination == NoteDestination::Local)
        {
            if let NoteAnchor::Line {
                side,
                line,
                start_line,
            } = &note.anchor
            {
                cards.push(PlacedCard {
                    card: CommentCard::Note(note.id),
                    path: note.path.clone(),
                    anchor: Some((*side, *line)),
                    start_line: *start_line,
                });
            }
        }
        cards
    }

    pub fn note(&self, id: NoteId) -> Option<&ReviewNote> {
        self.notes.iter().find(|note| note.id == id)
    }

    /// The notes going to `destination`.
    pub fn notes_to(&self, destination: NoteDestination) -> impl Iterator<Item = &ReviewNote> {
        self.notes
            .iter()
            .filter(move |note| note.destination == destination)
    }

    /// Parses every comment body not parsed yet (or changed since), and
    /// forgets the documents of comments that are gone.
    pub fn refresh_documents(&mut self) {
        let mut bodies: Vec<(String, &str)> = self
            .notes
            .iter()
            .map(|note| (note_key(note.id), note.body.as_str()))
            .collect();
        if let Some(state) = &self.github {
            bodies.extend(
                state
                    .threads
                    .iter()
                    .flat_map(|thread| &thread.comments)
                    .map(|comment| (comment.id.clone(), comment.body.as_str())),
            );
        }
        let mut documents = HashMap::new();
        for (key, body) in bodies {
            let document = match self.documents.remove(&key) {
                Some((parsed_body, document)) if parsed_body == body => document,
                Some(_) | None => Arc::new(MarkdownDocument::parse(body)),
            };
            documents.insert(key, (body.to_owned(), document));
        }
        self.documents = documents;
    }

    /// The parsed body of the comment with `key`.
    pub fn document(&self, key: &str) -> Option<Arc<MarkdownDocument>> {
        self.documents
            .get(key)
            .map(|(_, document)| document.clone())
    }
}

/// The document key of a note.
pub fn note_key(id: NoteId) -> String {
    format!("note-{id:?}")
}
