//! Authenticated Server-Sent Events stream.

use std::pin::Pin;
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use tokio::time::{Instant, timeout};

use crate::domain::GatewayEvent;

use super::{GatewayClient, GatewayError, SseDecoder};

/// Owned asynchronous gateway event stream.
pub type EventStream = Pin<Box<dyn Stream<Item = Result<GatewayEvent, GatewayError>> + Send>>;

impl GatewayClient {
    /// Open the authenticated event stream with optional channel filters.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] when the stream cannot connect.
    pub async fn events(&self, channels: &[String]) -> Result<EventStream, GatewayError> {
        let path = event_path(channels);
        let (response, request_id) =
            self.protected_get(&path, "events", "text/event-stream").await?;
        let mut bytes = response.bytes_stream();
        let stream = async_stream::try_stream! {
            let mut decoder = SseDecoder::default();
            while let Some(chunk) = bytes.next().await {
                let chunk = chunk.map_err(|error| stream_error(&request_id, error.to_string()))?;
                for event in decoder
                    .push(&chunk)
                    .map_err(|error| decode_error(&request_id, error.to_string()))?
                {
                    yield event;
                }
            }
            for event in decoder
                .finish()
                .map_err(|error| decode_error(&request_id, error.to_string()))?
            {
                yield event;
            }
        };
        Ok(Box::pin(stream))
    }

    /// Collect at most `limit` events within a caller-supplied duration.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] for connection or stream decoding failures.
    pub async fn collect_events(
        &self,
        channels: &[String],
        limit: usize,
        duration: Duration,
    ) -> Result<Vec<GatewayEvent>, GatewayError> {
        let mut stream = self.events(channels).await?;
        let deadline = Instant::now() + duration;
        let mut events = Vec::new();
        while events.len() < limit {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match timeout(remaining, stream.next()).await {
                Ok(Some(event)) => events.push(event?),
                Ok(None) | Err(_) => break,
            }
        }
        Ok(events)
    }
}

fn event_path(channels: &[String]) -> String {
    if channels.is_empty() {
        "/events".to_owned()
    } else {
        let joined = channels.join(",");
        let encoded = url::form_urlencoded::byte_serialize(joined.as_bytes()).collect::<String>();
        format!("/events?channels={encoded}")
    }
}

fn stream_error(request_id: &str, message: String) -> GatewayError {
    GatewayError::Transport {
        command: "events".to_owned(),
        request_id: request_id.to_owned(),
        message,
    }
}

fn decode_error(request_id: &str, message: String) -> GatewayError {
    GatewayError::Json { command: "events".to_owned(), request_id: request_id.to_owned(), message }
}
