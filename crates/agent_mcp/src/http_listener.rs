use std::{sync::Arc, time::Duration};

use axum::{Router, middleware};
use hyper::server::conn::http1;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use tokio::{net::TcpListener, sync::Semaphore, task::JoinSet};
use tokio_util::sync::CancellationToken;

use crate::{
    http_policy::{HttpPolicy, authorize_request},
    mcp_service::McpService,
};

const MAXIMUM_CONNECTIONS: usize = 32;
const MAXIMUM_HEADER_BYTES: usize = 16 * 1024;
const MAXIMUM_BODY_BYTES: usize = 1024 * 1024;
const HEADER_TIMEOUT: Duration = Duration::from_secs(5);

/// Serves bounded loopback HTTP connections until the owner closes the run.
pub(crate) async fn serve_listener(
    listener: TcpListener,
    service: McpService,
    policy: HttpPolicy,
    cancellation: CancellationToken,
) -> std::io::Result<()> {
    // Let the pinned SDK own all MCP parsing, negotiation and wire semantics.
    let configuration = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_sse_keep_alive(None)
        .with_allowed_hosts([listener.local_addr()?.to_string()])
        .enforce_origin_validation()
        .with_max_request_body_bytes(MAXIMUM_BODY_BYTES)
        .with_cancellation_token(cancellation.clone());
    let transport: StreamableHttpService<McpService, LocalSessionManager> =
        StreamableHttpService::new(move || Ok(service.clone()), Arc::default(), configuration);
    let router = Router::new()
        .route_service("/mcp", transport)
        .layer(middleware::from_fn_with_state(policy, authorize_request));

    // Bound sockets as well as handlers so incomplete headers cannot exhaust us.
    let connections = Arc::new(Semaphore::new(MAXIMUM_CONNECTIONS));
    let mut running = JoinSet::new();
    loop {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => break,
            Some(_) = running.join_next(), if !running.is_empty() => {},
            accepted = listener.accept() => {
                let (socket, _) = accepted?;
                let Ok(permit) = connections.clone().try_acquire_owned() else {
                    continue;
                };
                let router = router.clone();
                running.spawn(async move {
                    let _permit = permit;
                    let _ = http1::Builder::new()
                        .timer(TokioTimer::new())
                        .header_read_timeout(HEADER_TIMEOUT)
                        .max_buf_size(MAXIMUM_HEADER_BYTES)
                        .serve_connection(TokioIo::new(socket), TowerToHyperService::new(router))
                        .await;
                });
            }
        }
    }

    // Close every connection before returning; runtime teardown drains handlers.
    drop(listener);
    running.abort_all();
    while running.join_next().await.is_some() {}
    Ok(())
}
