use std::{
    net::{SocketAddr, TcpStream},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

use qd_agent_mcp::{McpHandler, McpServer, McpToolDescriptor, McpToolResult};
use serde_json::{Value, json};

use crate::support::{TestEndpoint, modern_request, post};

const TEST_TIMEOUT: Duration = Duration::from_secs(3);

/// A handler whose completion is controlled by the test's bounded channel.
struct DelayedHandler {
    started: mpsc::SyncSender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl McpHandler for DelayedHandler {
    fn tools(&self) -> Vec<McpToolDescriptor> {
        vec![McpToolDescriptor {
            name: "echo".into(),
            description: "Wait for the bounded synthetic test release.".into(),
            input_schema: json!({"type":"object"}),
            output_schema: None,
            read_only: true,
        }]
    }

    fn call_tool(&self, _name: &str, _arguments: Value) -> McpToolResult {
        self.started.send(()).expect("report handler start");
        self.release
            .lock()
            .unwrap()
            .recv_timeout(TEST_TIMEOUT)
            .expect("release synthetic handler");
        McpToolResult::success("completed", json!({"completed":true}))
    }
}

/// Shutdown and drop both close the listener, and explicit shutdown is idempotent.
#[test]
fn shutdown_and_drop_release_the_listener() {
    let mut endpoint = TestEndpoint::start();
    let address: SocketAddr = endpoint
        .server
        .endpoint()
        .trim_start_matches("http://")
        .trim_end_matches("/mcp")
        .parse()
        .unwrap();
    endpoint.server.shutdown().unwrap();
    endpoint.server.shutdown().unwrap();
    assert!(TcpStream::connect_timeout(&address, TEST_TIMEOUT).is_err());

    let endpoint = TestEndpoint::start();
    let address: SocketAddr = endpoint
        .server
        .endpoint()
        .trim_start_matches("http://")
        .trim_end_matches("/mcp")
        .parse()
        .unwrap();
    drop(endpoint);
    assert!(TcpStream::connect_timeout(&address, TEST_TIMEOUT).is_err());
}

/// Shutdown revokes connections immediately and joins an already executing handler.
#[test]
fn shutdown_drains_an_active_handler() {
    // Start a handler whose work cannot finish until the test releases it.
    let (started_sender, started_receiver) = mpsc::sync_channel(1);
    let (release_sender, release_receiver) = mpsc::sync_channel(1);
    let server = McpServer::start(Arc::new(DelayedHandler {
        started: started_sender,
        release: Mutex::new(release_receiver),
    }))
    .unwrap();
    let endpoint = server.endpoint().to_string();
    let token = server.token().to_string();
    let (request_finished_sender, request_finished_receiver) = mpsc::sync_channel(1);
    let request_thread = thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(TEST_TIMEOUT))
            .build()
            .into();
        let body = modern_request("tools/call", json!({"name":"echo","arguments":{}}));
        let _ = agent
            .post(endpoint)
            .header("Authorization", format!("Bearer {token}"))
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", "2026-07-28")
            .header("Mcp-Method", "tools/call")
            .header("Mcp-Name", "echo")
            .send_json(body);
        request_finished_sender.send(()).unwrap();
    });
    started_receiver.recv_timeout(TEST_TIMEOUT).unwrap();

    // The transport closes the response before waiting for synchronous work.
    let (shutdown_sender, shutdown_receiver) = mpsc::sync_channel(1);
    let shutdown_thread = thread::spawn(move || {
        drop(server);
        shutdown_sender.send(()).unwrap();
    });
    request_finished_receiver
        .recv_timeout(TEST_TIMEOUT)
        .unwrap();
    assert!(
        shutdown_receiver
            .recv_timeout(Duration::from_millis(50))
            .is_err()
    );

    // Once the handler finishes, the guard's owned runtime is fully joined.
    release_sender.send(()).unwrap();
    shutdown_receiver.recv_timeout(TEST_TIMEOUT).unwrap();
    request_thread.join().unwrap();
    shutdown_thread.join().unwrap();
}

/// Eight active tool calls exhaust request capacity without queueing more mutations.
#[test]
fn concurrent_requests_are_bounded() {
    let (started_sender, started_receiver) = mpsc::sync_channel(8);
    let (release_sender, release_receiver) = mpsc::sync_channel(8);
    let server = McpServer::start(Arc::new(DelayedHandler {
        started: started_sender,
        release: Mutex::new(release_receiver),
    }))
    .unwrap();
    let mut requests = Vec::new();
    for _ in 0..8 {
        let endpoint = server.endpoint().to_string();
        let token = server.token().to_string();
        requests.push(thread::spawn(move || {
            let body = modern_request("tools/call", json!({"name":"echo","arguments":{}}));
            post(
                &endpoint,
                Some(&token),
                &body.to_string(),
                &[
                    ("MCP-Protocol-Version", "2026-07-28"),
                    ("Mcp-Method", "tools/call"),
                    ("Mcp-Name", "echo"),
                ],
            )
        }));
    }
    for _ in 0..8 {
        started_receiver.recv_timeout(TEST_TIMEOUT).unwrap();
    }

    let body = modern_request("tools/call", json!({"name":"echo","arguments":{}}));
    let rejected = post(
        server.endpoint(),
        Some(server.token()),
        &body.to_string(),
        &[
            ("MCP-Protocol-Version", "2026-07-28"),
            ("Mcp-Method", "tools/call"),
            ("Mcp-Name", "echo"),
        ],
    );
    assert_eq!(rejected.status, 429);

    for _ in 0..8 {
        release_sender.send(()).unwrap();
    }
    for request in requests {
        assert_eq!(request.join().unwrap().status, 200);
    }
}
