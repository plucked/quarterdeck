use crate::{LineSide, ReviewEvent};

/// A review to submit: the verdict, its summary, new conversations on the
/// diff and replies to existing ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewSubmission {
    pub event: ReviewEvent,
    /// Markdown; may be empty when there are comments.
    pub body: String,
    pub threads: Vec<NewReviewThread>,
    pub replies: Vec<NewThreadReply>,
}

/// A new conversation on a line (or lines `start_line..=line`) of the diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewReviewThread {
    pub path: String,
    pub side: LineSide,
    pub line: u32,
    pub start_line: Option<u32>,
    pub body: String,
}

/// A reply to an existing conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewThreadReply {
    pub thread_id: String,
    pub body: String,
}

/// One part of a submission that GitHub took, reported as it happens so a
/// failure later does not post it twice on a retry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmittedPart {
    /// `threads[index]` is on GitHub.
    Thread(usize),
    /// `replies[index]` is on GitHub.
    Reply(usize),
}
