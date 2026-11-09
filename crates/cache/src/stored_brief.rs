use qd_studio::{BriefRecord, Studio};

/// A brief as stored: the checked studio and the record of how it was made
/// (without the raw answer and transcript, which are not kept).
#[derive(Debug, Clone)]
pub struct StoredBrief {
    pub studio: Studio,
    pub record: BriefRecord,
}
