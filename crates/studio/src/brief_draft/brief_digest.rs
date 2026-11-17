use std::fmt::Write as _;

use serde_json::Value;
use sha2::{Digest, Sha256};

/// JSON object keys are ordered canonically; array order remains meaningful.
pub(super) fn brief_digest(domain: &str, revision: u64, payload: &Value) -> String {
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update(revision.to_le_bytes());
    hash_value(&mut digest, payload);
    let mut result = String::with_capacity(64);
    for byte in digest.finalize() {
        let _ = write!(result, "{byte:02x}");
    }
    result
}

/// Typed delimiters and length prefixes prevent ambiguous field concatenation.
fn hash_value(digest: &mut Sha256, value: &Value) {
    match value {
        Value::Object(object) => {
            digest.update(b"object");
            digest.update(
                u64::try_from(object.len())
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            );
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            for key in keys {
                hash_text(digest, key);
                hash_value(digest, &object[key]);
            }
        }
        Value::Array(array) => {
            digest.update(b"array");
            digest.update(u64::try_from(array.len()).unwrap_or(u64::MAX).to_le_bytes());
            for item in array {
                hash_value(digest, item);
            }
        }
        Value::String(text) => {
            digest.update(b"string");
            hash_text(digest, text);
        }
        Value::Number(number) => {
            digest.update(b"number");
            hash_text(digest, &number.to_string());
        }
        Value::Bool(boolean) => digest.update([u8::from(*boolean)]),
        Value::Null => digest.update(b"null"),
    }
}

fn hash_text(digest: &mut Sha256, text: &str) {
    digest.update(u64::try_from(text.len()).unwrap_or(u64::MAX).to_le_bytes());
    digest.update(text.as_bytes());
}

/// Charges allocated JSON structure as well as text to the accepted-content budget.
pub(super) fn value_bytes(value: &Value) -> usize {
    size_of::<Value>()
        + match value {
            Value::String(text) => text.len(),
            Value::Array(array) => array.iter().map(value_bytes).sum(),
            Value::Object(object) => object
                .iter()
                .map(|(key, value)| size_of::<String>() + key.len() + value_bytes(value))
                .sum(),
            _ => 0,
        }
}
