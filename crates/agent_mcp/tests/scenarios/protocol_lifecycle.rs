use std::sync::atomic::Ordering;

use crate::support::{TestEndpoint, modern_request, post};
use serde_json::{Value, json};

/// Modern discovery, listing and tools carry complete results and private cache hints.
#[test]
fn modern_discovery_and_tool_lifecycle() {
    let endpoint = TestEndpoint::start();
    let discovery = endpoint.modern("server/discover", json!({}));
    assert_eq!(discovery["result"]["resultType"], "complete");
    assert_eq!(discovery["result"]["ttlMs"], 0);
    assert_eq!(discovery["result"]["cacheScope"], "private");
    assert!(
        discovery["result"]["supportedVersions"]
            .as_array()
            .unwrap()
            .contains(&json!("2026-07-28"))
    );

    let catalog = endpoint.modern("tools/list", json!({}));
    assert_eq!(catalog["result"]["tools"][0]["name"], "echo");
    assert_eq!(catalog["result"]["cacheScope"], "private");
    assert_eq!(catalog["result"]["ttlMs"], 0);
    assert_eq!(
        catalog["result"]["tools"][0]["annotations"]["readOnlyHint"],
        true
    );

    let result = endpoint.modern(
        "tools/call",
        json!({"name":"echo","arguments":{"value":"accepted"}}),
    );
    assert_eq!(result["result"]["resultType"], "complete");
    assert_eq!(result["result"]["structuredContent"]["value"], "accepted");
    assert_eq!(result["result"]["content"][0]["text"], "Accepted echo.");
    assert_eq!(endpoint.calls.load(Ordering::SeqCst), 1);
}

/// Modern direct calls work without a prior discovery or initialization request.
#[test]
fn modern_direct_call_and_repairable_handler_error() {
    let endpoint = TestEndpoint::start();
    let failure = endpoint.modern(
        "tools/call",
        json!({"name":"echo","arguments":{"value":"invalid"}}),
    );
    assert_eq!(failure["result"]["isError"], true);
    assert_eq!(failure["result"]["structuredContent"]["valid"], false);
    assert!(
        failure["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("repaired")
    );

    let success = endpoint.modern(
        "tools/call",
        json!({"name":"echo","arguments":{"value":"repaired"}}),
    );
    assert_eq!(success["result"]["isError"], false);
    assert_eq!(endpoint.calls.load(Ordering::SeqCst), 2);
}

/// Both promised legacy revisions initialize, acknowledge initialization and call tools.
#[test]
fn legacy_initialization_and_tools() {
    for version in ["2025-11-25", "2025-06-18"] {
        let endpoint = TestEndpoint::start();
        let initialize = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":version,"capabilities":{},"clientInfo":{"name":"legacy-test","version":"1"}
        }});
        let response = post(
            endpoint.server.endpoint(),
            Some(endpoint.server.token()),
            &initialize.to_string(),
            &[],
        );
        assert_eq!(response.status, 200, "{}", response.body);
        let body: Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["result"]["protocolVersion"], version);

        let notification = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        let response = post(
            endpoint.server.endpoint(),
            Some(endpoint.server.token()),
            &notification.to_string(),
            &[("MCP-Protocol-Version", version)],
        );
        assert_eq!(response.status, 202, "{}", response.body);

        for request in [
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
            json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"echo","arguments":{"value":"legacy"}}}),
        ] {
            let response = post(
                endpoint.server.endpoint(),
                Some(endpoint.server.token()),
                &request.to_string(),
                &[("MCP-Protocol-Version", version)],
            );
            assert_eq!(response.status, 200, "{}", response.body);
            let body: Value = serde_json::from_str(&response.body).unwrap();
            assert!(body.get("error").is_none(), "{body}");
            assert!(body["result"].get("resultType").is_none(), "{body}");
        }
        assert_eq!(endpoint.calls.load(Ordering::SeqCst), 1);
    }
}

/// Metadata and HTTP-header disagreement fail before application execution.
#[test]
fn protocol_and_header_mismatches_never_dispatch() {
    let endpoint = TestEndpoint::start();
    let request = modern_request("tools/call", json!({"name":"echo","arguments":{}}));
    for headers in [
        vec![
            ("MCP-Protocol-Version", "2026-07-28"),
            ("Mcp-Method", "tools/list"),
            ("Mcp-Name", "echo"),
        ],
        vec![
            ("MCP-Protocol-Version", "2026-07-28"),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "different"),
        ],
        vec![
            ("MCP-Protocol-Version", "2025-11-25"),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "echo"),
        ],
    ] {
        let response = post(
            endpoint.server.endpoint(),
            Some(endpoint.server.token()),
            &request.to_string(),
            &headers,
        );
        assert!(
            response.status >= 400 || response.body.contains("\"error\""),
            "{}",
            response.body
        );
    }
    let mut request = request;
    request["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"] = json!("2099-01-01");
    let response = post(
        endpoint.server.endpoint(),
        Some(endpoint.server.token()),
        &request.to_string(),
        &[
            ("MCP-Protocol-Version", "2099-01-01"),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "echo"),
        ],
    );
    let body: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(body["error"]["code"], -32022);
    assert!(
        body["error"]["data"]["supported"]
            .as_array()
            .unwrap()
            .contains(&json!("2026-07-28"))
    );
    assert_eq!(endpoint.calls.load(Ordering::SeqCst), 0);
}

/// Legacy defaults and known-but-unsupported revisions cannot bypass negotiation.
#[test]
fn unsupported_legacy_versions_never_dispatch() {
    let endpoint = TestEndpoint::start();
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{}}}).to_string();
    for headers in [vec![], vec![("MCP-Protocol-Version", "2025-03-26")]] {
        let response = post(
            endpoint.server.endpoint(),
            Some(endpoint.server.token()),
            &body,
            &headers,
        );
        let body: Value = serde_json::from_str(&response.body).unwrap();
        assert_eq!(body["error"]["code"], -32022, "{body}");
    }
    assert_eq!(endpoint.calls.load(Ordering::SeqCst), 0);
}
