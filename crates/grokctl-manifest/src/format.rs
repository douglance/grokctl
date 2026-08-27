//! Serializable host manifest types.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::policy::CommandPolicy;

/// Compatibility snapshot extracted from an authorized Grok Bot host.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostManifest {
    /// Host build or version identifier.
    pub host_version: String,
    /// Capabilities declared by the host.
    pub capabilities: Vec<String>,
    /// Gateway command names in host declaration order.
    pub commands: Vec<String>,
    /// Effect policy for every known command.
    pub policies: Vec<CommandPolicy>,
    /// Source and extraction notes.
    pub notes: ManifestNotes,
    /// SHA-256 of the host source used for extraction.
    pub source_sha256: String,
}

/// Provenance notes retained with a host manifest.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestNotes {
    /// How command names were obtained.
    pub commands: String,
    /// How capabilities were obtained.
    pub capabilities: String,
    /// Whether typed schemas came from the host.
    pub schemas: String,
}
