//! Manifest-to-host compatibility verdict.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use grokctl_manifest::HostManifest;

use crate::domain::HostStatus;

/// Live host version relationship to the manifest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionMatch {
    /// Versions are equal.
    Match,
    /// Versions differ; commands should warn and continue.
    Mismatch,
    /// Live host did not report a version.
    Unknown,
}

/// Compatibility comparison without secrets or command payloads.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatVerdict {
    /// Embedded or installed manifest version.
    pub pinned_version: String,
    /// Live host version.
    pub live_version: Option<String>,
    /// Version relationship.
    pub version_match: VersionMatch,
    /// Manifest capabilities absent from the host.
    pub missing_capabilities: Vec<String>,
    /// Live capabilities absent from the manifest.
    pub extra_capabilities: Vec<String>,
}

/// Compare live status with a manifest. Version mismatch never blocks calls.
#[must_use]
pub fn evaluate_compat(manifest: &HostManifest, live: &HostStatus) -> CompatVerdict {
    let version_match = live.host_version.as_ref().map_or(VersionMatch::Unknown, |version| {
        if version == &manifest.host_version { VersionMatch::Match } else { VersionMatch::Mismatch }
    });
    CompatVerdict {
        pinned_version: manifest.host_version.clone(),
        live_version: live.host_version.clone(),
        version_match,
        missing_capabilities: difference(&manifest.capabilities, &live.capabilities),
        extra_capabilities: difference(&live.capabilities, &manifest.capabilities),
    }
}

fn difference(left: &[String], right: &[String]) -> Vec<String> {
    let right = right.iter().collect::<BTreeSet<_>>();
    left.iter().filter(|value| !right.contains(value)).cloned().collect()
}

#[cfg(test)]
mod tests {
    use grokctl_manifest::{ManifestNotes, extract_host_manifest};

    use super::*;

    #[test]
    fn mismatch_warns_with_capability_differences() {
        let source = "var HOST_CAPABILITIES = [\"known\"];\nvar SAND_GATEWAY_COMMANDS = {\n  listAgents: x\n};\nvar SAND_GATEWAY_SLIM_COMMANDS = {};";
        let result = extract_host_manifest(source, "old");
        assert!(result.is_ok());
        let Some(mut manifest) = result.ok() else { return };
        manifest.notes = ManifestNotes {
            commands: String::new(),
            capabilities: String::new(),
            schemas: String::new(),
        };
        let verdict = evaluate_compat(
            &manifest,
            &HostStatus {
                host_version: Some("new".to_owned()),
                capabilities: vec!["extra".to_owned()],
            },
        );
        assert_eq!(verdict.version_match, VersionMatch::Mismatch);
        assert_eq!(verdict.missing_capabilities, ["known"]);
        assert_eq!(verdict.extra_capabilities, ["extra"]);
    }
}
