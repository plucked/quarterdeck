use std::sync::atomic::Ordering;

use serde_json::json;

use crate::support::{TestEndpoint, modern_request, post};

const MAXIMUM_BODY_BYTES: usize = 1024 * 1024;

/// Every request, including discovery and initialization, needs this run's token.
#[test]
fn authentication_covers_every_lifecycle_request() {
    let first = TestEndpoint::start();
    let second = TestEndpoint::start();
    assert_ne!(first.server.token(), second.server.token());
    for body in [
        modern_request("server/discover", json!({})),
        modern_request("tools/call", json!({"name":"echo","arguments":{}})),
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}
        }}),
    ] {
        for token in [None, Some("incorrect"), Some(second.server.token())] {
            let response = post(first.server.endpoint(), token, &body.to_string(), &[]);
            assert_eq!(response.status, 401);
        }
    }
    assert_eq!(first.calls.load(Ordering::SeqCst), 0);
    assert_eq!(second.calls.load(Ordering::SeqCst), 0);
}

/// Host authority and Origin are rejected independently before tool execution.
#[test]
fn exact_authority_and_absent_origin_are_required() {
    let endpoint = TestEndpoint::start();
    let body = modern_request("tools/call", json!({"name":"echo","arguments":{}})).to_string();
    for headers in [
        vec![("Host", "attacker.invalid")],
        vec![("Host", "127.0.0.1")],
        vec![("Origin", "https://attacker.invalid")],
        vec![("Origin", "null")],
        vec![(
            "Origin",
            endpoint.server.endpoint().trim_end_matches("/mcp"),
        )],
    ] {
        let response = post(
            endpoint.server.endpoint(),
            Some(endpoint.server.token()),
            &body,
            &headers,
        );
        assert_eq!(response.status, 403, "{}", response.body);
    }
    assert_eq!(endpoint.calls.load(Ordering::SeqCst), 0);

    let result = endpoint.modern("tools/call", json!({"name":"echo","arguments":{}}));
    assert_eq!(result["result"]["isError"], false);
}

/// Only the exact endpoint path is accepted and config contains no secret values.
#[test]
fn path_and_configuration_are_scoped_to_one_run() {
    let endpoint = TestEndpoint::start();
    for suffix in ["/nested", "?extra=1", "/"] {
        let response = post(
            &format!("{}{suffix}", endpoint.server.endpoint()),
            Some(endpoint.server.token()),
            "{}",
            &[],
        );
        assert_eq!(response.status, 404);
    }
    let configuration = endpoint.server.config_json();
    assert!(!configuration.contains(endpoint.server.token()));
    assert!(!configuration.contains(endpoint.server.endpoint()));
    assert!(configuration.contains("${QUARTERDECK_MCP_TOKEN}"));
    assert!(configuration.contains("${QUARTERDECK_MCP_URL}"));
    assert_eq!(endpoint.calls.load(Ordering::SeqCst), 0);
}

/// Malformed, excessively nested and oversized bodies never execute a handler.
#[test]
fn malformed_and_bounded_bodies_are_rejected() {
    let endpoint = TestEndpoint::start();
    let deep = format!("{}0{}", "[".repeat(200), "]".repeat(200));
    for body in ["{".to_string(), deep, "[]".into(), "null".into()] {
        let response = post(
            endpoint.server.endpoint(),
            Some(endpoint.server.token()),
            &body,
            &[],
        );
        assert!(
            response.status >= 400 || response.body.contains("\"error\""),
            "{}",
            response.body
        );
    }
    let oversized = " ".repeat(MAXIMUM_BODY_BYTES + 1);
    let response = post(
        endpoint.server.endpoint(),
        Some(endpoint.server.token()),
        &oversized,
        &[],
    );
    assert_eq!(response.status, 413, "{}", response.body);
    assert_eq!(endpoint.calls.load(Ordering::SeqCst), 0);
}

/// Modern required headers and metadata cannot be omitted to bypass SDK checks.
#[test]
fn missing_modern_protocol_signals_are_rejected() {
    let endpoint = TestEndpoint::start();
    let body = modern_request("tools/call", json!({"name":"echo","arguments":{}}));
    for headers in [
        vec![],
        vec![("MCP-Protocol-Version", "2026-07-28")],
        vec![
            ("MCP-Protocol-Version", "2026-07-28"),
            ("Mcp-Method", "tools/call"),
        ],
    ] {
        let response = post(
            endpoint.server.endpoint(),
            Some(endpoint.server.token()),
            &body.to_string(),
            &headers,
        );
        assert!(
            response.status >= 400 || response.body.contains("\"error\""),
            "{}",
            response.body
        );
    }
    let mut body = body;
    body["params"].as_object_mut().unwrap().remove("_meta");
    let response = post(
        endpoint.server.endpoint(),
        Some(endpoint.server.token()),
        &body.to_string(),
        &[
            ("MCP-Protocol-Version", "2026-07-28"),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "echo"),
        ],
    );
    assert!(
        response.status >= 400 || response.body.contains("\"error\""),
        "{}",
        response.body
    );
    assert_eq!(endpoint.calls.load(Ordering::SeqCst), 0);
}
