//! Named gateway profile values.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// One named gateway connection profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    /// Gateway URL.
    pub url: String,
    /// Optional file containing only the bearer token.
    pub token_file: Option<PathBuf>,
    /// Permit plaintext HTTP to a non-loopback host.
    #[serde(default)]
    pub allow_insecure_http: bool,
}

/// Serializable named profile collection.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Profiles {
    /// Default profile name.
    pub current: Option<String>,
    /// Profile values keyed by name.
    pub profiles: BTreeMap<String, Profile>,
}
