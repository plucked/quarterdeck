use qd_github::LineSide;

use super::comment_card::CommentCard;

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
