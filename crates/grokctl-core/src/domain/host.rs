//! Host compatibility status.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Relevant fields returned by `getHostStatus`.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    /// Live host version, when exposed.
    pub host_version: Option<String>,
    /// Live capability names.
    #[serde(default)]
    pub capabilities: Vec<String>,
}
