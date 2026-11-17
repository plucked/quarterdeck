use serde_json::{Value, json};

/// Page offsets count Unicode scalars; encoded replies remain bounded even for wide text.
pub(crate) const MAX_PAGE_CHARACTERS: usize = 16 * 1024;
const MAX_DIRECT_BYTES: usize = 48 * 1024;

/// Returns a typed candidate when it fits, otherwise lossless canonical JSON fragments.
/// `identity` contains the stable artifact-unit ID and immutable revision for this read.
pub(crate) fn mcp_json_page(
    candidate: &Value,
    mut identity: Value,
    offset: usize,
    limit: usize,
) -> Value {
    let encoded = candidate.to_string();
    let total_characters = encoded.chars().count();
    let limit = limit.clamp(1, MAX_PAGE_CHARACTERS);
    if offset == 0 && total_characters <= limit && encoded.len() <= MAX_DIRECT_BYTES {
        identity["candidate"] = candidate.clone();
        identity["encoding"] = json!("json_value");
        identity["next_offset"] = Value::Null;
        identity["next_step"] = json!(
            "This is the complete typed candidate. Submit its full replacement with the latest expected_revision and a fresh operation_id."
        );
        return identity;
    }

    // Character offsets cannot split UTF-8; matching revisions detect edits between reads.
    let fragment: String = encoded.chars().skip(offset).take(limit).collect();
    let next = offset.saturating_add(fragment.chars().count());
    identity["candidate_json_fragment"] = json!(fragment);
    identity["candidate_json_total_bytes"] = json!(encoded.len());
    identity["total_characters"] = json!(total_characters);
    identity["encoding"] = json!("json");
    identity["offset_unit"] = json!("unicode_scalar");
    identity["offset"] = json!(offset);
    identity["next_offset"] = json!((next < total_characters).then_some(next));
    identity["next_step"] = json!(
        "Concatenate candidate_json_fragment from successive next_offset pages, then parse that JSON to recover the complete candidate. Keep one revision across pages; restart at offset 0 if it changes. Submit replacements atomically, within 900 KiB encoded candidate JSON."
    );
    identity
}
