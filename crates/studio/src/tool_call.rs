use serde_json::Value;

/// The most characters of a tool call's input kept.
const MAX_INPUT_CHARS: usize = 200;

/// One tool call an agent made while writing a brief, such as reading a
/// file: the tool and its input as readable text (`file_path: head/src/lib.rs`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub tool: String,
    pub input: String,
}

impl ToolCall {
    /// The tool calls in `transcript` (stream-json lines), in order, but the
    /// structured answer itself.
    pub fn from_transcript(transcript: &[String]) -> Vec<Self> {
        transcript
            .iter()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|event| event.get("type").and_then(Value::as_str) == Some("assistant"))
            .filter_map(|event| event.pointer("/message/content")?.as_array().cloned())
            .flatten()
            .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_use"))
            .filter_map(|block| {
                let tool = block.get("name").and_then(Value::as_str)?;
                (tool != "StructuredOutput").then(|| Self {
                    tool: tool.to_owned(),
                    input: readable_input(block.get("input")),
                })
            })
            .collect()
    }
}

/// A tool input object as `key: value, key: value`, cut to a readable length.
fn readable_input(input: Option<&Value>) -> String {
    let Some(Value::Object(fields)) = input else {
        return String::new();
    };
    let text = fields
        .iter()
        .map(|(key, value)| match value {
            Value::String(text) => format!("{key}: {text}"),
            other => format!("{key}: {other}"),
        })
        .collect::<Vec<_>>()
        .join(", ");
    text.chars().take(MAX_INPUT_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tool calls are read with their input; the answer tool is not.
    #[test]
    fn reads_the_tool_calls() {
        let transcript = vec![
            r#"{"type":"system","subtype":"init"}"#.to_owned(),
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Grep","input":{"pattern":"retry_after","path":"head"}}]}}"#.to_owned(),
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"StructuredOutput","input":{}}]}}"#.to_owned(),
        ];
        assert_eq!(
            ToolCall::from_transcript(&transcript),
            [ToolCall {
                tool: "Grep".into(),
                input: "pattern: retry_after, path: head".into(),
            }]
        );
    }
}
