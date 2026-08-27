//! Local-only test helpers for Grokctl.

use std::collections::VecDeque;
use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// One request observed by the local fake gateway.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordedRequest {
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Authorization header, when present.
    pub authorization: Option<String>,
    /// Sand request ID header, when present.
    pub request_id: Option<String>,
    /// Slim-avatar request header, when present.
    pub slim_avatars: Option<String>,
    /// Desktop gateway network token header, when present.
    pub network_token: Option<String>,
    /// Parsed JSON request body.
    pub body: Option<Value>,
}

/// One queued fake gateway response.
#[derive(Clone, Debug)]
pub struct MockResponse {
    /// HTTP status code.
    pub status: u16,
    /// JSON response body.
    pub body: Value,
}

#[derive(Debug)]
struct MockState {
    api_responses: Mutex<VecDeque<MockResponse>>,
    requests: Mutex<Vec<RecordedRequest>>,
}

/// In-process HTTP gateway used only by tests.
#[derive(Debug)]
pub struct MockGateway {
    /// Loopback origin of the fake gateway.
    pub base_url: String,
    state: Arc<MockState>,
    task: JoinHandle<()>,
}

impl MockGateway {
    /// Start a fake gateway on an ephemeral loopback port.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the test host cannot bind or inspect a loopback socket.
    pub async fn start(api_response: Value) -> Result<Self, std::io::Error> {
        Self::start_responses(vec![MockResponse { status: 200, body: api_response }]).await
    }

    /// Start a fake gateway with ordered API responses.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the test host cannot bind or inspect a loopback socket.
    pub async fn start_responses(responses: Vec<MockResponse>) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let state = Arc::new(MockState {
            api_responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        });
        let app = router(Arc::clone(&state));
        let task = tokio::spawn(async move {
            let _result = axum::serve(listener, app).await;
        });
        Ok(Self { base_url: format!("http://{address}"), state, task })
    }

    /// Return a snapshot of recorded requests.
    pub async fn requests(&self) -> Vec<RecordedRequest> {
        self.state.requests.lock().await.clone()
    }
}

impl Drop for MockGateway {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn router(state: Arc<MockState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/events", get(events))
        .route("/avatars/{bot_id}", get(avatar))
        .route("/api/{command}", post(command))
        .with_state(state)
}

async fn health(State(state): State<Arc<MockState>>, headers: HeaderMap) -> impl IntoResponse {
    record(&state, "GET", "/health", &headers, None).await;
    (
        StatusCode::OK,
        axum::Json(json!({
            "ok": true,
            "isBusy": false,
            "activeAgentId": null,
            "startedAt": 1
        })),
    )
}

async fn events(State(state): State<Arc<MockState>>, headers: HeaderMap) -> impl IntoResponse {
    record(&state, "GET", "/events", &headers, None).await;
    let mut response_headers = HeaderMap::new();
    response_headers.insert("content-type", HeaderValue::from_static("text/event-stream"));
    (
        response_headers,
        "retry: 1000\n\ndata: {\"channel\":\"agents\",\"payload\":{\"count\":1}}\n\n",
    )
}

async fn avatar(
    State(state): State<Arc<MockState>>,
    Path(bot_id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    record(&state, "GET", &format!("/avatars/{bot_id}"), &headers, None).await;
    let mut response_headers = HeaderMap::new();
    response_headers.insert("content-type", HeaderValue::from_static("image/png"));
    response_headers.insert("etag", HeaderValue::from_static("\"avatar-v1\""));
    (response_headers, vec![137, 80, 78, 71])
}

async fn command(
    State(state): State<Arc<MockState>>,
    Path(command): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let parsed = serde_json::from_slice(&body).ok();
    record(&state, "POST", &format!("/api/{command}"), &headers, parsed).await;
    let response =
        state.api_responses.lock().await.pop_front().unwrap_or(MockResponse {
            status: 500,
            body: json!({ "error": "no queued response" }),
        });
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, axum::Json(response.body))
}

async fn record(
    state: &MockState,
    method: &str,
    path: &str,
    headers: &HeaderMap,
    body: Option<Value>,
) {
    state.requests.lock().await.push(RecordedRequest {
        method: method.to_owned(),
        path: path.to_owned(),
        authorization: header(headers, "authorization"),
        request_id: header(headers, "x-sand-request-id"),
        slim_avatars: header(headers, "x-sand-slim-avatars"),
        network_token: header(headers, "x-sand-network-token"),
        body,
    });
}

fn header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers.get(name).and_then(|value| value.to_str().ok()).map(str::to_owned)
}
