//! Deterministic host source extraction.

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::format::{HostManifest, ManifestNotes};
use crate::markers::{CAPABILITIES_START, COMMANDS_END, COMMANDS_START};
use crate::policy::classify_command;

/// Failure while extracting a compatibility manifest.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum ExtractError {
    /// Host version was empty or contained multiple lines.
    #[error("host version must be one non-empty line")]
    InvalidVersion,
    /// Expected command markers were absent or out of order.
    #[error("host gateway command table was not found")]
    CommandsMissing,
    /// No command names could be extracted.
    #[error("host gateway command table was empty")]
    CommandsEmpty,
    /// Expected capability markers were absent or unclosed.
    #[error("host capabilities declaration was not found")]
    CapabilitiesMissing,
}

/// Extract a manifest from `host-main.cjs` source and the host version file.
///
/// # Errors
///
/// Returns [`ExtractError`] when the version or source markers are invalid.
pub fn extract_host_manifest(
    host_main_source: &str,
    host_version_text: &str,
) -> Result<HostManifest, ExtractError> {
    let host_version = parse_version(host_version_text)?;
    let commands = extract_commands(host_main_source)?;
    let capabilities = extract_capabilities(host_main_source)?;
    let policies = commands.iter().map(|name| classify_command(name)).collect();
    let source_sha256 = hex_digest(host_main_source.as_bytes());
    Ok(HostManifest {
        host_version,
        capabilities,
        commands,
        policies,
        notes: ManifestNotes {
            commands: "Extracted from SAND_GATEWAY_COMMANDS keys.".to_owned(),
            capabilities: "Extracted from HOST_CAPABILITIES.".to_owned(),
            schemas: "The host exposes no per-command JSON schema table.".to_owned(),
        },
        source_sha256,
    })
}

fn parse_version(text: &str) -> Result<String, ExtractError> {
    let value = text.trim();
    if value.is_empty() || value.contains('\n') {
        return Err(ExtractError::InvalidVersion);
    }
    Ok(value.to_owned())
}

fn extract_commands(source: &str) -> Result<Vec<String>, ExtractError> {
    let start = source.find(COMMANDS_START).ok_or(ExtractError::CommandsMissing)?;
    let end = source.find(COMMANDS_END).ok_or(ExtractError::CommandsMissing)?;
    if end <= start {
        return Err(ExtractError::CommandsMissing);
    }
    let commands = source[start + COMMANDS_START.len()..end]
        .lines()
        .filter_map(command_name)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if commands.is_empty() {
        return Err(ExtractError::CommandsEmpty);
    }
    Ok(commands)
}

fn command_name(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let (candidate, _) = trimmed.split_once(':')?;
    (!candidate.is_empty() && candidate.chars().all(|character| character.is_ascii_alphanumeric()))
        .then_some(candidate)
}

fn extract_capabilities(source: &str) -> Result<Vec<String>, ExtractError> {
    let start = source.find(CAPABILITIES_START).ok_or(ExtractError::CapabilitiesMissing)?;
    let remainder = &source[start + CAPABILITIES_START.len()..];
    let end = remainder.find("];").ok_or(ExtractError::CapabilitiesMissing)?;
    let capabilities = remainder[..end]
        .split(',')
        .filter_map(|token| capability_value(source, token.trim()))
        .collect::<Vec<_>>();
    if capabilities.is_empty() {
        return Err(ExtractError::CapabilitiesMissing);
    }
    Ok(capabilities)
}

fn capability_value(source: &str, token: &str) -> Option<String> {
    quoted(token).map(str::to_owned).or_else(|| resolve_string_constant(source, token))
}

fn quoted(value: &str) -> Option<&str> {
    value.strip_prefix('"')?.strip_suffix('"')
}

fn resolve_string_constant(source: &str, identifier: &str) -> Option<String> {
    let prefix = format!("var {identifier} = \"");
    let remainder = source.split_once(&prefix)?.1;
    Some(remainder.split_once('"')?.0.to_owned())
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str = r#"
var ORDERED = "orderedReplicasV1";
var HOST_CAPABILITIES = [ORDERED, "sendAcceptanceV1"];
var SAND_GATEWAY_COMMANDS = {
  listAgents: (api) => api.listAgents(),
  sendPrompt: (api, body) => api.sendPrompt(body),
  resetForeverBox: (api, body) => api.resetForeverBox(body)
};
var SAND_GATEWAY_SLIM_COMMANDS = { ...SAND_GATEWAY_COMMANDS };
"#;

    #[test]
    fn extracts_commands_capabilities_and_source_digest() {
        let result = extract_host_manifest(HOST, "0e82340\n");
        assert!(result.is_ok(), "fixture must be valid: {result:?}");
        let Some(manifest) = result.ok() else { return };

        assert_eq!(manifest.commands, ["listAgents", "sendPrompt", "resetForeverBox"]);
        assert_eq!(manifest.capabilities, ["orderedReplicasV1", "sendAcceptanceV1"]);
        assert_eq!(manifest.source_sha256.len(), 64);
    }

    #[test]
    fn rejects_missing_command_table() {
        let error = extract_host_manifest("var HOST_CAPABILITIES = [];", "v1").err();

        assert_eq!(error, Some(ExtractError::CommandsMissing));
    }
}
