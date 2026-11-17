use anyhow::{Result, ensure};
use qd_studio::GuidedToolAudit;

const MAXIMUM_AUDIT_ENTRIES: usize = 512;
const MAXIMUM_ISSUE_CODES: usize = 64;
const MAXIMUM_IDENTITY_BYTES: usize = 2048;
const MAXIMUM_METADATA_BYTES: usize = 512;
const MAXIMUM_AUDIT_TEXT_BYTES: usize = 128;

/// A durable receipt and bounded run metadata saved atomically with its review artifact.
///
/// The receipt comes from the sealed server draft and successful CLI completion.
/// Audit records contain typed tool outcomes and issue codes, never transcripts,
/// endpoint URLs, bearer credentials or serialized agent messages.
#[derive(Debug, Clone)]
pub struct AgentPublication {
    pub run_id: String,
    pub snapshot_id: String,
    pub validation_receipt: String,
    pub cost_usd: Option<f64>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub duration_ms: u64,
    pub audit: Vec<GuidedToolAudit>,
}

impl AgentPublication {
    /// Rejects unbounded metadata before claiming the publication gate.
    pub(crate) fn validate(&self) -> Result<()> {
        // Required identities must remain small, explicit and nonempty.
        for value in [&self.run_id, &self.snapshot_id, &self.validation_receipt] {
            ensure!(
                !value.is_empty() && value.len() <= MAXIMUM_IDENTITY_BYTES,
                "agent publication identity exceeds its size bounds"
            );
        }
        ensure!(
            self.cost_usd
                .is_none_or(|cost| cost.is_finite() && cost >= 0.0),
            "agent cost must be finite and nonnegative"
        );
        for value in [&self.session_id, &self.model].into_iter().flatten() {
            ensure!(
                value.len() <= MAXIMUM_METADATA_BYTES,
                "agent model or session metadata is too long"
            );
        }

        // Match the handler's bounded audit window without silently dropping data.
        ensure!(
            self.audit.len() <= MAXIMUM_AUDIT_ENTRIES,
            "agent audit contains too many events"
        );
        for event in &self.audit {
            ensure!(
                event.issue_codes.len() <= MAXIMUM_ISSUE_CODES,
                "agent audit contains too many issue codes"
            );
            for value in [&event.tool_name, &event.status]
                .into_iter()
                .chain(&event.issue_codes)
            {
                ensure!(
                    !value.is_empty() && value.len() <= MAXIMUM_AUDIT_TEXT_BYTES,
                    "agent audit field exceeds its size bounds"
                );
            }
        }
        Ok(())
    }

    /// An idempotent publication retry must reproduce all persisted metadata.
    pub(crate) fn matches_publication(&self, other: &Self) -> bool {
        self.run_id == other.run_id
            && self.snapshot_id == other.snapshot_id
            && self.validation_receipt == other.validation_receipt
            && self.cost_usd.map(f64::to_bits) == other.cost_usd.map(f64::to_bits)
            && self.session_id == other.session_id
            && self.model == other.model
            && self.duration_ms == other.duration_ms
            && self.audit.len() == other.audit.len()
            && self.audit.iter().zip(&other.audit).all(|(left, right)| {
                left.tool_name == right.tool_name
                    && left.status == right.status
                    && left.revision == right.revision
                    && left.issue_codes == right.issue_codes
            })
    }
}
