use std::sync::OnceLock;

use serde_json::Value;

use super::{BriefIssue, BriefIssues};

/// Checks precisely the schema constructs used by the compiled Brief contract.
pub(crate) fn check_page_schema(candidate: &Value, issues: &mut BriefIssues) {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| {
        serde_json::from_str(crate::STUDIO_SCHEMA).expect("the compiled Brief schema is valid JSON")
    });
    check_shape(
        candidate,
        &schema["properties"]["pages"]["items"],
        schema,
        "",
        issues,
    );
}

/// Object keys and required nullable fields cannot be silently discarded by serde.
fn check_shape(
    value: &Value,
    schema: &Value,
    root: &Value,
    location: &str,
    issues: &mut BriefIssues,
) {
    if let Some(reference) = schema["$ref"].as_str() {
        if let Some(definition) = reference
            .strip_prefix('#')
            .and_then(|pointer| root.pointer(pointer))
        {
            check_shape(value, definition, root, location, issues);
        }
        return;
    }
    if let Some(choices) = schema["anyOf"].as_array() {
        if let Some(choice) = choices
            .iter()
            .find(|choice| matches_choice(value, choice, root))
        {
            check_shape(value, choice, root, location, issues);
        } else {
            issues.push(BriefIssue::new(
                "invalid_shape",
                location,
                "This value has no supported widget kind or nullable field type.",
            ));
        }
        return;
    }
    if let Some(kind) = schema["type"].as_str()
        && !matches_type(value, kind)
    {
        issues.push(BriefIssue::new(
            "invalid_shape",
            location,
            &format!("Expected {kind}."),
        ));
        return;
    }
    if let Some(choices) = schema["enum"].as_array()
        && !choices.contains(value)
    {
        issues.push(BriefIssue::new(
            "invalid_shape",
            location,
            &format!(
                "Use one of these values: {}.",
                Value::Array(choices.clone())
            ),
        ));
    }
    match value {
        Value::Object(object) => {
            if let Some(required) = schema["required"].as_array() {
                for field in required
                    .iter()
                    .filter_map(Value::as_str)
                    .filter(|field| !object.contains_key(*field))
                {
                    issues.push(BriefIssue::new(
                        "missing_field",
                        &pointer(location, field),
                        "This field is required, even when its value may be null.",
                    ));
                }
            }
            for (field, child) in object {
                let child_location = pointer(location, field);
                match schema["properties"].get(field) {
                    Some(child_schema) => {
                        check_shape(child, child_schema, root, &child_location, issues);
                    }
                    None if schema["additionalProperties"] == false => {
                        issues.push(BriefIssue::new(
                            "unknown_field",
                            &child_location,
                            "This field is not part of the Brief schema and would be discarded.",
                        ));
                    }
                    None => {}
                }
            }
        }
        Value::Array(array) => {
            if let Some(item_schema) = schema.get("items") {
                for (index, child) in array.iter().enumerate() {
                    check_shape(
                        child,
                        item_schema,
                        root,
                        &format!("{location}/{index}"),
                        issues,
                    );
                }
            }
        }
        _ => {}
    }
}

/// Widget unions have a kind discriminator; all other unions are nullable values.
fn matches_choice(value: &Value, choice: &Value, root: &Value) -> bool {
    let schema = choice["$ref"]
        .as_str()
        .and_then(|reference| reference.strip_prefix('#'))
        .and_then(|pointer| root.pointer(pointer))
        .unwrap_or(choice);
    if let Some(kinds) = schema["properties"]["kind"]["enum"].as_array() {
        return kinds.contains(&value["kind"]);
    }
    schema["type"]
        .as_str()
        .is_some_and(|kind| matches_type(value, kind))
}

fn matches_type(value: &Value, kind: &str) -> bool {
    match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.is_i64() || value.is_u64(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => false,
    }
}

fn pointer(parent: &str, field: &str) -> String {
    format!("{parent}/{}", field.replace('~', "~0").replace('/', "~1"))
}
