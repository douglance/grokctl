//! Stable gateway error shape.

use thiserror::Error;

/// Gateway request or response failure.
#[derive(Debug, Error)]
pub enum GatewayError {
    /// HTTP transport failed.
    #[error("gateway transport failed for {command} ({request_id}): {message}")]
    Transport {
        /// Command or endpoint name.
        command: String,
        /// Client request identifier.
        request_id: String,
        /// Redacted transport message.
        message: String,
    },
    /// Gateway returned a non-success status.
    #[error("gateway returned HTTP {status} for {command} ({request_id}): {message}")]
    Status {
        /// HTTP status.
        status: u16,
        /// Command or endpoint name.
        command: String,
        /// Client request identifier.
        request_id: String,
        /// Redacted gateway message.
        message: String,
    },
    /// Protected request lacked a bearer token.
    #[error("gateway token is required for {0}")]
    MissingToken(String),
    /// Response JSON was invalid.
    #[error("gateway returned invalid JSON for {command} ({request_id}): {message}")]
    Json {
        /// Command name.
        command: String,
        /// Request identifier.
        request_id: String,
        /// Parse error without secrets.
        message: String,
    },
}
