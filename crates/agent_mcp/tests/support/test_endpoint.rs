use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use qd_agent_mcp::{McpHandler, McpServer, McpToolDescriptor, McpToolResult};
use serde_json::{Value, json};

const HTTP_TIMEOUT: Duration = Duration::from_secs(3);

/// An isolated server exposing an echo tool with a deterministic repair error.
pub struct TestEndpoint {
    pub server: McpServer,
    pub calls: Arc<AtomicUsize>,
}

/// A synthetic application handler, independent of any review-domain data.
struct EchoHandler {
    calls: Arc<AtomicUsize>,
}

impl McpHandler for EchoHandler {
    fn tools(&self) -> Vec<McpToolDescriptor> {
        vec![McpToolDescriptor {
            name: "echo".into(),
            description: "Echo a value, or return a repairable validation error.".into(),
            input_schema: json!({"type":"object","properties":{"value":{"type":"string"}}}),
            output_schema: None,
            read_only: true,
        }]
    }

    fn call_tool(&self, _name: &str, arguments: Value) -> McpToolResult {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if arguments["value"] == "invalid" {
            return McpToolResult::failure(
                "Change value from invalid to repaired.",
                json!({"valid":false}),
            );
        }
        McpToolResult::success("Accepted echo.", arguments)
    }
}

impl TestEndpoint {
    /// Starts a fresh loopback endpoint with a visible execution count.
    pub fn start() -> Self {
        let calls = Arc::new(AtomicUsize::new(0));
        let handler = Arc::new(EchoHandler {
            calls: calls.clone(),
        });
        let server = McpServer::start(handler).expect("start synthetic MCP server");
        Self { server, calls }
    }

    /// Sends a complete modern request using this run's authorization.
    pub fn modern(&self, method: &str, arguments: Value) -> Value {
        let request = modern_request(method, arguments);
        let mut headers = vec![
            ("MCP-Protocol-Version", "2026-07-28"),
            ("Mcp-Method", method),
        ];
        if method == "tools/call" {
            headers.push(("Mcp-Name", "echo"));
        }
        let response = post(
            self.server.endpoint(),
            Some(self.server.token()),
            &request.to_string(),
            &headers,
        );
        assert_eq!(response.status, 200, "{}", response.body);
        serde_json::from_str(&response.body).expect("JSON MCP response")
    }
}

/// A complete HTTP result kept small enough for readable assertion failures.
pub struct HttpReply {
    pub status: u16,
    pub body: String,
}

/// Makes one bounded local request, optionally authenticated and with extra headers.
pub fn post(
    endpoint: &str,
    token: Option<&str>,
    body: &str,
    headers: &[(&str, &str)],
) -> HttpReply {
    // A global timeout ensures malformed-server tests cannot hang the test runner.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(HTTP_TIMEOUT))
        .http_status_as_error(false)
        .build()
        .into();
    let mut request = agent
        .post(endpoint)
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream");
    if let Some(token) = token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    for (name, value) in headers {
        request = request.header(*name, *value);
    }

    // Read the complete body while the same request timeout remains in force.
    let mut response = request.send(body).expect("send local HTTP request");
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .read_to_string()
        .expect("read local response");
    HttpReply { status, body }
}

/// Adds the modern inline lifecycle metadata independently of SDK serialization.
pub fn modern_request(method: &str, mut arguments: Value) -> Value {
    arguments["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientInfo": {"name":"transport-test","version":"1"},
        "io.modelcontextprotocol/clientCapabilities": {}
    });
    json!({"jsonrpc":"2.0","id":1,"method":method,"params":arguments})
}
