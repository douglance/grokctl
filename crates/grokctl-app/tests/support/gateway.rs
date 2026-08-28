use std::collections::VecDeque;
use std::error::Error;
use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

#[derive(Clone, Debug)]
pub struct MockResponse {
    pub status: u16,
    pub body: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordedRequest {
    pub path: String,
    pub body: Option<Value>,
}

#[derive(Debug)]
struct MockState {
    responses: Mutex<VecDeque<MockResponse>>,
    requests: Mutex<Vec<RecordedRequest>>,
}

pub struct MockGateway {
    pub base_url: String,
    state: Arc<MockState>,
    task: JoinHandle<()>,
}

impl MockGateway {
    pub async fn start_responses(responses: Vec<MockResponse>) -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let state = Arc::new(MockState {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        });
        let app =
            Router::new().route("/api/{command}", post(command)).with_state(Arc::clone(&state));
        let task = tokio::spawn(async move {
            let _result = axum::serve(listener, app).await;
        });
        Ok(Self { base_url: format!("http://{address}"), state, task })
    }

    pub async fn requests(&self) -> Vec<RecordedRequest> {
        self.state.requests.lock().await.clone()
    }
}

impl Drop for MockGateway {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn command(
    State(state): State<Arc<MockState>>,
    Path(command): Path<String>,
    body: Bytes,
) -> impl IntoResponse {
    state.requests.lock().await.push(RecordedRequest {
        path: format!("/api/{command}"),
        body: serde_json::from_slice(&body).ok(),
    });
    let response = state.responses.lock().await.pop_front().unwrap_or(MockResponse {
        status: 500,
        body: serde_json::json!({ "error": "no queued response" }),
    });
    let status = StatusCode::from_u16(response.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, axum::Json(response.body))
}
