use std::{
    net::TcpListener,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use tokio_util::sync::CancellationToken;

use crate::{
    McpError, McpHandler, http_listener::serve_listener, http_policy::HttpPolicy,
    mcp_service::McpService, run_credential::RunCredential,
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const MAXIMUM_BLOCKING_THREADS: usize = 8;

/// Owns an ephemeral MCP endpoint and its dedicated private runtime thread.
///
/// Start, shut down and drop this guard on a background worker. Shutdown waits
/// for active synchronous handlers; they must return promptly. The endpoint and
/// bearer capability must never be written to reports or application logs.
pub struct McpServer {
    endpoint: String,
    credential: RunCredential,
    cancellation: CancellationToken,
    thread: Option<thread::JoinHandle<Result<(), McpError>>>,
}

impl McpServer {
    /// Starts listening before returning so the child can connect immediately.
    pub fn start(handler: Arc<dyn McpHandler>) -> Result<Self, McpError> {
        // Validate the run catalog and create a fresh local capability first.
        let cancellation = CancellationToken::new();
        let service = McpService::new(handler, cancellation.clone())?;
        let credential = RunCredential::generate()?;
        let listener = TcpListener::bind("127.0.0.1:0")?;
        listener.set_nonblocking(true)?;
        let authority = listener.local_addr()?.to_string();
        let endpoint = format!("http://{authority}/mcp");
        let policy = HttpPolicy::new(authority, credential.as_str(), cancellation.clone());

        // Construct and drive Tokio exclusively on the owned transport thread.
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let thread_cancellation = cancellation.clone();
        let thread = thread::Builder::new()
            .name("quarterdeck-mcp".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .max_blocking_threads(MAXIMUM_BLOCKING_THREADS)
                    .build()?;
                runtime.block_on(async move {
                    let listener = tokio::net::TcpListener::from_std(listener)?;
                    let _ = ready_sender.send(());
                    serve_listener(listener, service, policy, thread_cancellation).await
                })?;
                // Dropping the runtime joins the short synchronous tool handlers.
                Ok(())
            })?;
        let mut server = Self {
            endpoint,
            credential,
            cancellation,
            thread: Some(thread),
        };

        // Do not expose an endpoint whose runtime failed to start.
        if ready_receiver.recv_timeout(STARTUP_TIMEOUT).is_err() {
            server.shutdown()?;
            return Err(McpError::RuntimeStopped);
        }
        Ok(server)
    }

    /// Returns the run's already-listening loopback URL.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Returns the per-run secret for the authorized child's environment only.
    pub fn token(&self) -> &str {
        self.credential.as_str()
    }

    /// Returns a Claude MCP configuration containing environment placeholders only.
    pub fn config_json(&self) -> String {
        serde_json::json!({
            "mcpServers": {
                "quarterdeck_run": {
                    "type": "http",
                    "url": "${QUARTERDECK_MCP_URL}",
                    "headers": { "Authorization": "Bearer ${QUARTERDECK_MCP_TOKEN}" }
                }
            }
        })
        .to_string()
    }

    /// Revokes transport access, closes sockets and joins all active handlers.
    ///
    /// Repeated calls are harmless. Close application mutations first when
    /// cancelling a run, because an already executing handler cannot be undone.
    pub fn shutdown(&mut self) -> Result<(), McpError> {
        self.cancellation.cancel();
        match self.thread.take() {
            Some(thread) => thread.join().map_err(|_| McpError::RuntimeStopped)?,
            None => Ok(()),
        }
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
