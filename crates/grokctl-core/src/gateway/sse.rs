//! Incremental Server-Sent Events decoding.

use thiserror::Error;

use crate::domain::GatewayEvent;

/// Invalid SSE data frame.
#[derive(Debug, Error)]
pub enum SseError {
    /// Event data was not valid JSON.
    #[error("gateway event data is invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// Stateful SSE decoder for arbitrary byte chunk boundaries.
#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: String,
}

impl SseDecoder {
    /// Decode a chunk and return every complete gateway event.
    ///
    /// # Errors
    ///
    /// Returns [`SseError`] when a complete data frame contains invalid JSON.
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<GatewayEvent>, SseError> {
        self.buffer.push_str(&String::from_utf8_lossy(chunk));
        self.buffer = self.buffer.replace("\r\n", "\n").replace('\r', "\n");
        let mut events = Vec::new();
        while let Some(block) = self.take_block() {
            events.extend(decode_block(&block)?);
        }
        Ok(events)
    }

    /// Decode a final unterminated frame, when present.
    ///
    /// # Errors
    ///
    /// Returns [`SseError`] when the trailing frame contains invalid JSON.
    pub fn finish(&mut self) -> Result<Vec<GatewayEvent>, SseError> {
        let block = std::mem::take(&mut self.buffer);
        Ok(decode_block(&block)?.into_iter().collect())
    }

    fn take_block(&mut self) -> Option<String> {
        let end = self.buffer.find("\n\n")?;
        let block = self.buffer[..end].to_owned();
        self.buffer.drain(..end + 2);
        Some(block)
    }
}

fn decode_block(block: &str) -> Result<Option<GatewayEvent>, SseError> {
    let data = block
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(|line| line.strip_prefix(' ').unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    if data.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&data)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_split_crlf_multiline_and_heartbeat_frames() {
        let mut decoder = SseDecoder::default();
        let first = decoder.push(b"retry: 1000\r\n\r\n:pi");
        assert!(first.is_ok());
        let second = decoder.push(
            b"ng\r\n\r\ndata: {\"channel\":\"agents\",\r\ndata: \"payload\":{\"count\":2}}\r\n\r\n",
        );
        assert!(second.is_ok(), "valid event must decode: {second:?}");
        let Some(events) = second.ok() else { return };
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].channel, "agents");
        assert_eq!(events[0].payload["count"], 2);
    }
}
