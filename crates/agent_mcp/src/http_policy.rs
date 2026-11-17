use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Request, State},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

const MAXIMUM_REQUESTS: usize = 8;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Immutable request policy shared by independently executing HTTP requests.
#[derive(Clone)]
pub(crate) struct HttpPolicy {
    authority: Arc<str>,
    authorization: Arc<str>,
    requests: Arc<Semaphore>,
    cancellation: CancellationToken,
}

impl HttpPolicy {
    /// Restricts a run to its exact loopback authority and bearer capability.
    pub(crate) fn new(authority: String, token: &str, cancellation: CancellationToken) -> Self {
        Self {
            authority: authority.into(),
            authorization: format!("Bearer {token}").into(),
            requests: Arc::new(Semaphore::new(MAXIMUM_REQUESTS)),
            cancellation,
        }
    }

    /// Rejects unauthorized and browser-originated requests before SDK parsing.
    fn validate_request(&self, request: &Request) -> Result<(), StatusCode> {
        // Authenticate every path and method, including discovery and initialization.
        if single_header(request.headers(), header::AUTHORIZATION)
            != Some(self.authorization.as_ref())
        {
            return Err(StatusCode::UNAUTHORIZED);
        }
        if self.cancellation.is_cancelled() {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }

        // A browser origin is never needed by the authorized command-line client.
        if single_header(request.headers(), header::HOST) != Some(self.authority.as_ref())
            || request.headers().contains_key(header::ORIGIN)
        {
            return Err(StatusCode::FORBIDDEN);
        }
        if request.uri().path() != "/mcp" || request.uri().query().is_some() {
            return Err(StatusCode::NOT_FOUND);
        }
        if !matches!(
            *request.method(),
            Method::POST | Method::GET | Method::DELETE
        ) {
            return Err(StatusCode::METHOD_NOT_ALLOWED);
        }
        Ok(())
    }
}

/// Applies authorization and bounds the lifetime of one HTTP operation.
pub(crate) async fn authorize_request(
    State(policy): State<HttpPolicy>,
    request: Request,
    next: Next,
) -> Response {
    // Reject unsafe headers before reading attacker-controlled request bodies.
    if let Err(status) = policy.validate_request(&request) {
        return status.into_response();
    }
    let Ok(_permit) = policy.requests.clone().try_acquire_owned() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };

    // This timeout covers body reading and SDK dispatch, not the whole agent run.
    let response = tokio::select! {
        biased;
        () = policy.cancellation.cancelled() => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        result = tokio::time::timeout(REQUEST_TIMEOUT, next.run(request)) => {
            result.unwrap_or_else(|_| StatusCode::REQUEST_TIMEOUT.into_response())
        }
    };
    response
}

/// Reads exactly one unambiguous, valid text header value.
fn single_header(headers: &HeaderMap, name: header::HeaderName) -> Option<&str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Some(value)
}
