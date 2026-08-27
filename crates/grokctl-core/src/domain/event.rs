//! Gateway event envelope.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One event emitted by `GET /events`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct GatewayEvent {
    /// Event channel name.
    pub channel: String,
    /// Channel-specific event data.
    pub payload: Value,
}
