use gpui::{AnyElement, IntoElement, SharedString, div, prelude::*};

use crate::controls::NumberFlow;
use crate::theme;

/// How the last operation on GitHub or the clipboard went: a review
/// submission, an edit on GitHub, a copy, a merge or a close.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OperationStatus {
    #[default]
    Idle,
    /// It runs; the message says what it does.
    Working(SharedString),
    /// It worked; the message says what happened.
    Done(SharedString),
    /// It failed; the message says why.
    Failed(SharedString),
}

impl OperationStatus {
    /// One small line saying how it goes, colored by the outcome; nothing while idle.
    #[track_caller]
    pub fn render(&self) -> Option<AnyElement> {
        let (text, color) = match self {
            Self::Idle => return None,
            Self::Working(text) => (text.clone(), theme::accent()),
            Self::Done(text) => (text.clone(), theme::success()),
            Self::Failed(text) => (text.clone(), theme::danger()),
        };
        Some(
            div()
                .text_xs()
                .text_color(color)
                .child(NumberFlow::label(text))
                .into_any_element(),
        )
    }
}
