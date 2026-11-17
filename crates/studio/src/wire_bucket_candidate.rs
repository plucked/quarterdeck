use serde::Deserialize;
use serde_json::{Value, json};

use crate::guided_review::{BucketCandidate, GuideIssue, GuideIssueCode, ItemCandidate};

const MAX_WIRE_BYTES: usize = 900 * 1024;
const BUCKET_WIRE_OVERHEAD: usize =
    r#"{"label":,"description":,"suggest_viewed":,"items":[]}"#.len();
const ITEM_WIRE_OVERHEAD: usize = r#"{"label":,"description":,"paths":[]}"#.len();

/// The MCP bucket shape, decoded privately before entering the typed draft.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WireBucketCandidate {
    label: String,
    description: String,
    suggest_viewed: bool,
    items: Vec<WireItemCandidate>,
}

/// MCP reading-item fields reject unknown keys independently of domain validation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireItemCandidate {
    label: String,
    description: String,
    paths: Vec<String>,
}

impl WireBucketCandidate {
    /// Moves the decoded proposal into transport-independent domain values.
    pub(crate) fn into_candidate(self) -> BucketCandidate {
        BucketCandidate {
            label: self.label,
            description: self.description,
            suggest_viewed: self.suggest_viewed,
            items: self
                .items
                .into_iter()
                .map(|item| ItemCandidate {
                    label: item.label,
                    description: item.description,
                    paths: item.paths,
                })
                .collect(),
        }
    }
}

/// Bounds compact MCP candidate JSON after the draft has resolved operation replay.
pub(crate) fn candidate_wire_issue(candidate: &BucketCandidate) -> Option<GuideIssue> {
    (candidate_wire_bytes(candidate) > MAX_WIRE_BYTES).then(|| {
        GuideIssue::new(
            GuideIssueCode::ResourceLimit,
            "The candidate exceeds 900 KiB encoded JSON. Split it into smaller complete buckets.",
        )
    })
}

/// Encodes accepted selectors for complete or fragmented recovery reads.
pub(crate) fn candidate_json(candidate: &BucketCandidate) -> Value {
    json!({"label":candidate.label,"description":candidate.description,"suggest_viewed":candidate.suggest_viewed,
        "items":candidate.items.iter().map(|item| json!({"label":item.label,"description":item.description,"paths":item.paths})).collect::<Vec<_>>()})
}

/// Includes fields, separators and escaped text without allocating an encoded candidate.
fn candidate_wire_bytes(candidate: &BucketCandidate) -> usize {
    // Fixed punctuation and scalar fields match the recovery representation exactly.
    let fields = BUCKET_WIRE_OVERHEAD
        + json_string_bytes(&candidate.label)
        + json_string_bytes(&candidate.description)
        + "true".len()
        + usize::from(!candidate.suggest_viewed)
        + candidate.items.len().saturating_sub(1);

    // Item and path array commas count even when individual selectors are empty.
    candidate.items.iter().fold(fields, |bytes, item| {
        let fields = ITEM_WIRE_OVERHEAD
            + json_string_bytes(&item.label)
            + json_string_bytes(&item.description)
            + item.paths.len().saturating_sub(1);
        item.paths
            .iter()
            .fold(bytes.saturating_add(fields), |bytes, path| {
                bytes.saturating_add(json_string_bytes(path))
            })
    })
}

/// Exact compact JSON string size, including quotes, escapes and unchanged UTF-8 bytes.
fn json_string_bytes(text: &str) -> usize {
    text.bytes().fold(2_usize, |bytes, byte| {
        let encoded = match byte {
            b'"' | b'\\' | b'\n' | b'\r' | b'\t' | 8 | 12 => 2,
            0..=31 => 6,
            _ => 1,
        };
        bytes.saturating_add(encoded)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Accounting matches recovery JSON for escapes, UTF-8, booleans and array separators.
    #[test]
    fn encoded_bytes_match_the_actual_candidate_wire_shape() {
        let text: String = (0..=31)
            .filter_map(char::from_u32)
            .chain("😀é\\\"/".chars())
            .collect();
        for suggest_viewed in [false, true] {
            for item_count in [0, 1, 2] {
                for paths in [
                    vec![],
                    vec![String::new()],
                    vec![text.clone(), "file.rs".into()],
                ] {
                    let candidate = BucketCandidate {
                        label: text.clone(),
                        description: text.clone(),
                        suggest_viewed,
                        items: vec![
                            ItemCandidate {
                                label: text.clone(),
                                description: text.clone(),
                                paths,
                            };
                            item_count
                        ],
                    };
                    let wire = candidate_json(&candidate);
                    assert_eq!(candidate_wire_bytes(&candidate), wire.to_string().len());
                    let decoded: WireBucketCandidate = serde_json::from_value(wire).unwrap();
                    assert_eq!(decoded.into_candidate(), candidate);
                }
            }
        }
    }
}
