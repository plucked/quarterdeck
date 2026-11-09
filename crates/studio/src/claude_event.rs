use serde_json::Value;

/// One line of Claude Code's `stream-json` output, as far as a run cares.
#[derive(Debug, PartialEq)]
pub(crate) enum ClaudeEvent {
    /// The session started (`system` / `init`), with the model it runs.
    Started { model: Option<String> },
    /// A chunk of reasoning or text.
    Thinking,
    /// A tool call starts: the tool's name. The structured answer is the
    /// input of the `StructuredOutput` tool.
    ToolStarted(String),
    /// A chunk of the current tool call's input, this many characters long.
    InputChunk(usize),
    /// The run ended (`result`).
    Finished(ClaudeResult),
    /// Anything else (whole messages, hooks, rate-limit notes).
    Other,
}

/// The final `result` line.
#[derive(Debug, PartialEq)]
pub(crate) struct ClaudeResult {
    pub is_error: bool,
    pub structured_output: Option<Value>,
    pub cost_usd: Option<f64>,
    pub session_id: Option<String>,
    /// The result text, or the subtype when there is none (`error_max_turns`).
    pub message: String,
}

impl ClaudeEvent {
    /// The event a line stands for; lines that are not JSON are `Other`.
    pub(crate) fn parse(line: &str) -> Self {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Self::Other;
        };
        let text_at = |pointer: &str| value.pointer(pointer).and_then(Value::as_str);
        match (text_at("/type"), text_at("/subtype")) {
            (Some("system"), Some("init")) => Self::Started {
                model: text_at("/model").map(str::to_owned),
            },
            (Some("stream_event"), _) => Self::of_stream_event(&value),
            (Some("result"), subtype) => Self::Finished(ClaudeResult {
                is_error: value
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                structured_output: value
                    .get("structured_output")
                    .filter(|output| !output.is_null())
                    .cloned(),
                cost_usd: value.get("total_cost_usd").and_then(Value::as_f64),
                session_id: text_at("/session_id").map(str::to_owned),
                message: text_at("/result")
                    .filter(|result| !result.is_empty())
                    .or(subtype)
                    .unwrap_or_default()
                    .to_owned(),
            }),
            _ => Self::Other,
        }
    }

    /// A partial message: reasoning, a tool call starting, or a piece of a
    /// tool's input.
    fn of_stream_event(value: &Value) -> Self {
        let text_at = |pointer: &str| value.pointer(pointer).and_then(Value::as_str);
        if text_at("/event/type") == Some("content_block_start")
            && text_at("/event/content_block/type") == Some("tool_use")
        {
            return Self::ToolStarted(
                text_at("/event/content_block/name")
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
        match text_at("/event/delta/type") {
            Some("input_json_delta") => Self::InputChunk(
                value
                    .pointer("/event/delta/partial_json")
                    .and_then(Value::as_str)
                    .map_or(0, str::len),
            ),
            Some("thinking_delta" | "text_delta") => Self::Thinking,
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The init line, tool starts, input chunks and reasoning chunks are told apart.
    #[test]
    fn reads_progress_lines() {
        assert_eq!(
            ClaudeEvent::parse(
                r#"{"type":"system","subtype":"init","tools":[],"model":"claude-opus-5-5"}"#
            ),
            ClaudeEvent::Started {
                model: Some("claude-opus-5-5".into())
            }
        );
        assert_eq!(
            ClaudeEvent::parse(
                r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"le"}}}"#
            ),
            ClaudeEvent::InputChunk(4)
        );
        assert_eq!(
            ClaudeEvent::parse(
                r#"{"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"t","name":"Grep","input":{}}}}"#
            ),
            ClaudeEvent::ToolStarted("Grep".into())
        );
        assert_eq!(
            ClaudeEvent::parse(
                r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"…"}}}"#
            ),
            ClaudeEvent::Thinking
        );
        assert_eq!(ClaudeEvent::parse("not json"), ClaudeEvent::Other);
    }

    /// The result line carries the structured output, cost and session.
    #[test]
    fn reads_the_result() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,"result":"","structured_output":{"lead":"x","widgets":[]},"total_cost_usd":0.25,"session_id":"abc"}"#;
        let ClaudeEvent::Finished(result) = ClaudeEvent::parse(line) else {
            panic!("not a result");
        };
        assert!(!result.is_error);
        assert_eq!(result.cost_usd, Some(0.25));
        assert_eq!(result.session_id.as_deref(), Some("abc"));
        assert_eq!(result.message, "success");
        assert!(result.structured_output.is_some());
    }
}
