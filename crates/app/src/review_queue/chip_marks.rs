use std::collections::HashMap;

use gpui::{AnyElement, IntoElement, div, prelude::*, px, rgb};
use qd_github::PullRequest;

use super::others_facet::OthersFacet;
use crate::avatar_images::Avatar;

/// An author's avatar in a chip.
const CHIP_AVATAR_SIZE: f32 = 14.;
/// A label's color dot in a chip.
const LABEL_DOT_SIZE: f32 = 8.;

/// What a chip shows before its name: an author's avatar, a label's color.
pub struct ChipMarks {
    /// Avatar URL by login.
    avatars: HashMap<String, String>,
    /// GitHub's hex color by label name.
    label_colors: HashMap<String, String>,
}

impl ChipMarks {
    /// The avatars and label colors of `pull_requests`.
    pub fn of(pull_requests: &[&PullRequest]) -> Self {
        let mut marks = Self {
            avatars: HashMap::new(),
            label_colors: HashMap::new(),
        };
        for pull_request in pull_requests.iter().copied() {
            marks
                .avatars
                .entry(pull_request.author.login.clone())
                .or_insert_with(|| pull_request.author.avatar_url.clone());
            for label in &pull_request.labels {
                marks
                    .label_colors
                    .entry(label.name.clone())
                    .or_insert_with(|| label.color.clone());
            }
        }
        marks
    }

    /// The mark before `value`'s chip in `facet`'s row, when known.
    pub fn mark_of(&self, facet: OthersFacet, value: &str) -> Option<AnyElement> {
        match facet {
            OthersFacet::Authors => self
                .avatars
                .get(value)
                .map(|url| Avatar::new(url.clone(), px(CHIP_AVATAR_SIZE)).into_any_element()),
            OthersFacet::Labels => {
                let color = u32::from_str_radix(self.label_colors.get(value)?, 16).ok()?;
                Some(
                    div()
                        .flex_none()
                        .size(px(LABEL_DOT_SIZE))
                        .rounded_full()
                        .bg(rgb(color))
                        .into_any_element(),
                )
            }
        }
    }
}
