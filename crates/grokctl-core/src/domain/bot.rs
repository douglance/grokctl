//! Bot roster and prompt types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Bot row returned by `listAgents`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BotSummary {
    /// Stable Bot identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Whether this row represents a group.
    #[serde(default)]
    pub is_group: bool,
    /// Whether a turn is running.
    #[serde(default)]
    pub is_running: bool,
    /// Whether the Bot is composing a message.
    #[serde(default)]
    pub is_composing_message: bool,
    /// Widget or input request that awaits the user.
    pub awaiting_user_response: Option<serde_json::Value>,
}

/// Immediate `sendPrompt` acknowledgement.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct PromptAccepted {
    /// Whether the host accepted the prompt.
    pub accepted: bool,
}
