//! Host health response.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Response from `GET /health`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    /// Whether the host reports healthy.
    pub ok: bool,
    /// Whether any Bot is busy.
    #[serde(default)]
    pub is_busy: bool,
    /// Active Bot identifier.
    pub active_agent_id: Option<String>,
    /// Host start epoch milliseconds.
    #[serde(default)]
    pub started_at: u64,
}
