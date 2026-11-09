use std::sync::Arc;

use qd_markdown::MarkdownDocument;
use qd_studio::{CodeAnchor, StepWeight};

/// A reading-order step with its explanation parsed.
pub struct PreparedStep {
    pub title: String,
    pub explanation: Arc<MarkdownDocument>,
    pub weight: StepWeight,
    pub anchor: Option<CodeAnchor>,
}
