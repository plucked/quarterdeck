use std::ops::RangeInclusive;

use crate::CodeSide;

const MAX_MESSAGE_CHARACTERS: usize = 1024;
const MAX_LOCATION_CHARACTERS: usize = 512;
const MAX_SHOWN_RANGES: usize = 32;

/// One repair location, optionally carrying the exact citable ranges for its anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefIssue {
    pub code: String,
    pub page_id: Option<String>,
    pub widget_index: Option<usize>,
    pub location: String,
    pub message: String,
    pub path: Option<String>,
    pub side: Option<CodeSide>,
    pub allowed_ranges: Vec<RangeInclusive<u32>>,
    pub range_count: usize,
}

impl BriefIssue {
    /// Bounds diagnostic text even when an invalid field or identifier is enormous.
    pub(crate) fn new(code: &str, location: &str, message: &str) -> Self {
        let location: String = location.chars().take(MAX_LOCATION_CHARACTERS).collect();
        let widget_index = location
            .strip_prefix("/widgets/")
            .and_then(|tail| tail.split('/').next())
            .and_then(|index| index.parse().ok());
        Self {
            code: code.into(),
            page_id: None,
            widget_index,
            location,
            message: message.chars().take(MAX_MESSAGE_CHARACTERS).collect(),
            path: None,
            side: None,
            allowed_ranges: Vec::new(),
            range_count: 0,
        }
    }

    pub(crate) fn with_ranges(
        mut self,
        path: &str,
        side: CodeSide,
        ranges: Vec<RangeInclusive<u32>>,
    ) -> Self {
        self.path = Some(path.chars().take(MAX_LOCATION_CHARACTERS).collect());
        self.side = Some(side);
        self.range_count = ranges.len();
        self.allowed_ranges = ranges.into_iter().take(MAX_SHOWN_RANGES).collect();
        self
    }

    /// Escaped-text accounting leaves room for keys, indexes and range pairs.
    pub(crate) fn storage_bytes(&self) -> usize {
        2048 + 6
            * (self.code.len()
                + self.location.len()
                + self.message.len()
                + self.page_id.as_ref().map_or(0, String::len)
                + self.path.as_ref().map_or(0, String::len))
    }
}
