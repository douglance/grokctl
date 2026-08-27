//! Mutation receipt records.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Durable local mutation state.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptState {
    /// Request was admitted but has not completed.
    Started,
    /// Gateway returned success.
    Succeeded,
    /// Request may have reached the host but no response was observed.
    Ambiguous,
    /// Operator verified that the mutation was applied.
    ResolvedApplied,
    /// Operator verified that the mutation was not applied.
    ResolvedNotApplied,
}

/// Secret-free idempotency receipt.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationReceipt {
    /// Gateway identity digest.
    pub gateway_id: String,
    /// Host command name.
    pub command: String,
    /// Caller idempotency key.
    pub idempotency_key: String,
    /// Canonical input digest.
    pub input_hash: String,
    /// Current receipt state.
    pub state: ReceiptState,
    /// Request identifier, when sent.
    pub request_id: Option<String>,
    /// Response digest, when completed.
    pub response_hash: Option<String>,
}
