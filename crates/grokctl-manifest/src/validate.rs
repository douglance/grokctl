//! Manifest consistency checks.

use std::collections::HashSet;

use thiserror::Error;

use crate::format::HostManifest;

/// Invalid manifest failure.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum ManifestError {
    /// Host version is empty.
    #[error("manifest host version is empty")]
    EmptyVersion,
    /// A command occurs more than once.
    #[error("manifest command is duplicated: {0}")]
    DuplicateCommand(String),
    /// Policy names do not exactly match command names.
    #[error("manifest policy does not cover every command")]
    PolicyCoverage,
    /// Source digest is not lowercase SHA-256 hex.
    #[error("manifest source digest is invalid")]
    InvalidDigest,
}

/// Validate a host manifest before embedding or installing it.
///
/// # Errors
///
/// Returns [`ManifestError`] for malformed or incomplete data.
pub fn validate_manifest(manifest: &HostManifest) -> Result<(), ManifestError> {
    if manifest.host_version.trim().is_empty() {
        return Err(ManifestError::EmptyVersion);
    }
    let commands = unique_commands(&manifest.commands)?;
    let policies: HashSet<&str> = manifest.policies.iter().map(|row| row.name.as_str()).collect();
    if commands != policies {
        return Err(ManifestError::PolicyCoverage);
    }
    if manifest.source_sha256.len() != 64
        || !manifest.source_sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(ManifestError::InvalidDigest);
    }
    Ok(())
}

fn unique_commands(commands: &[String]) -> Result<HashSet<&str>, ManifestError> {
    let mut seen = HashSet::with_capacity(commands.len());
    for command in commands {
        if !seen.insert(command.as_str()) {
            return Err(ManifestError::DuplicateCommand(command.clone()));
        }
    }
    Ok(seen)
}
