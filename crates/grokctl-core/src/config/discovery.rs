//! Environment, profile, and gateway file discovery.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use super::desktop::{DesktopConnection, discover_desktop};
use super::{GatewaySecret, GatewayUrlError, normalize_gateway_url};

/// On-host `gateway.json` record.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryFile {
    /// Bound TCP port.
    #[serde(default)]
    pub port: Option<u16>,
    /// Explicit gateway origin for an imported desktop connection.
    pub url: Option<String>,
    /// Host process ID for diagnostics only.
    #[serde(default)]
    pub pid: u32,
    /// Host start epoch in milliseconds.
    #[serde(default)]
    pub started_at: u64,
    /// HTTP scheme.
    #[serde(default = "default_scheme")]
    pub scheme: String,
    /// Bound host.
    #[serde(default = "default_host")]
    pub host: String,
    /// Optional bearer token.
    pub token: Option<String>,
    /// Optional protected gateway headers.
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Inputs used to resolve a gateway without mutating process environment.
#[derive(Clone, Debug, Default)]
pub struct DiscoverOptions {
    /// Explicit URL with highest precedence.
    pub url: Option<String>,
    /// Explicit token with highest precedence.
    pub token: Option<String>,
    /// Discovery file override.
    pub discovery_path: Option<PathBuf>,
    /// Environment snapshot.
    pub env: HashMap<String, String>,
    /// Permit plaintext HTTP to a non-loopback host.
    pub allow_insecure_http: bool,
}

/// Secret-safe resolved gateway connection.
#[derive(Clone, Debug)]
pub struct ResolvedGateway {
    /// Normalized gateway origin.
    pub base_url: Url,
    /// Runtime-only bearer token.
    pub token: Option<GatewaySecret>,
    /// Runtime-only headers required by a desktop gateway proxy.
    pub headers: HashMap<String, GatewaySecret>,
    /// Source discovery file, when one was read.
    pub discovery_path: Option<PathBuf>,
    /// Whether a token is available.
    pub has_token: bool,
}

/// Gateway discovery failure.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    /// Discovery file could not be read.
    #[error("could not read gateway discovery file: {0}")]
    Read(#[from] std::io::Error),
    /// Discovery JSON was invalid.
    #[error("gateway discovery file is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// Desktop descriptor or keychain access failed.
    #[error("{0}")]
    Desktop(String),
    /// Gateway URL was invalid.
    #[error(transparent)]
    Url(#[from] GatewayUrlError),
    /// No gateway URL or discovery port was available.
    #[error("gateway URL was not found")]
    Missing,
}

/// Resolve the gateway from explicit options, environment, and `gateway.json`.
///
/// # Errors
///
/// Returns [`DiscoveryError`] when configuration is absent or malformed.
pub fn discover_gateway(options: &DiscoverOptions) -> Result<ResolvedGateway, DiscoveryError> {
    let path = options.discovery_path.clone().or_else(default_discovery_path);
    let file = path.as_deref().and_then(read_optional_file).transpose()?;
    let desktop = desktop_fallback(options, file.as_ref())?;
    let raw_url = explicit_url(options)
        .or_else(|| file.as_ref().and_then(file_url))
        .or_else(|| desktop.as_ref().map(|row| row.connection.base_url.clone()));
    let base_url = normalize_gateway_url(
        raw_url.as_deref().ok_or(DiscoveryError::Missing)?,
        options.allow_insecure_http,
    )?;
    let token = explicit_token(options)
        .or_else(|| file.as_ref().and_then(|row| row.token.clone()))
        .or_else(|| desktop.as_ref().and_then(|row| row.connection.token.clone()));
    let headers = file_headers(file.as_ref())
        .unwrap_or_else(|| desktop_headers(desktop.as_ref().map(|row| &row.connection)));
    let has_token = token.as_ref().is_some_and(|value| !value.is_empty());
    Ok(ResolvedGateway {
        base_url,
        token: token.filter(|value| !value.is_empty()).map(GatewaySecret::new),
        headers,
        discovery_path: path
            .filter(|candidate| candidate.exists())
            .or_else(|| desktop.map(|row| row.path)),
        has_token,
    })
}

fn desktop_fallback(
    options: &DiscoverOptions,
    file: Option<&DiscoveryFile>,
) -> Result<Option<super::desktop::DesktopGateway>, DiscoveryError> {
    if explicit_url(options).is_some() || file.is_some() || options.discovery_path.is_some() {
        return Ok(None);
    }
    discover_desktop().map_err(|error| DiscoveryError::Desktop(error.to_string()))
}

fn desktop_headers(connection: Option<&DesktopConnection>) -> HashMap<String, GatewaySecret> {
    connection.map_or_else(HashMap::new, |row| {
        row.headers
            .iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(name, value)| (name.clone(), GatewaySecret::new(value.clone())))
            .collect()
    })
}

fn explicit_url(options: &DiscoverOptions) -> Option<String> {
    options
        .url
        .clone()
        .or_else(|| options.env.get("GROKCTL_GATEWAY_URL").cloned())
        .or_else(|| options.env.get("GROKBOT_GATEWAY_URL").cloned())
        .or_else(|| options.env.get("SAND_GATEWAY_URL").cloned())
}

fn explicit_token(options: &DiscoverOptions) -> Option<String> {
    options
        .token
        .clone()
        .or_else(|| options.env.get("GROKCTL_GATEWAY_TOKEN").cloned())
        .or_else(|| options.env.get("SAND_GATEWAY_TOKEN").cloned())
}

fn read_optional_file(path: &Path) -> Option<Result<DiscoveryFile, DiscoveryError>> {
    path.exists().then(|| {
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    })
}

fn file_url(file: &DiscoveryFile) -> Option<String> {
    if let Some(url) = &file.url {
        return Some(url.clone());
    }
    let host = match file.host.as_str() {
        "0.0.0.0" | "::" | "[::]" => "127.0.0.1",
        value => value,
    };
    Some(format!("{}://{}:{}", file.scheme, host, file.port?))
}

fn file_headers(file: Option<&DiscoveryFile>) -> Option<HashMap<String, GatewaySecret>> {
    file.filter(|row| !row.headers.is_empty()).map(|row| {
        row.headers
            .iter()
            .filter(|(_, value)| !value.is_empty())
            .map(|(name, value)| (name.clone(), GatewaySecret::new(value.clone())))
            .collect()
    })
}

fn default_discovery_path() -> Option<PathBuf> {
    let primary = PathBuf::from("/home/box/sand-data/gateway.json");
    primary.exists().then_some(primary).or_else(|| {
        let alias = PathBuf::from("/home/box/agent-data/gateway.json");
        alias.exists().then_some(alias)
    })
}

fn default_scheme() -> String {
    "http".to_owned()
}

fn default_host() -> String {
    "127.0.0.1".to_owned()
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn explicit_url_and_token_override_discovery_file() {
        let result = fixture_options();
        assert!(result.is_ok(), "fixture discovery must succeed: {result:?}");
        let Some(gateway) = result.ok() else { return };
        assert_eq!(gateway.base_url.as_str(), "https://remote.example/");
        assert!(gateway.has_token);
        assert_eq!(format!("{:?}", gateway.token), "Some(GatewaySecret([redacted]))");
    }

    fn fixture_options() -> Result<ResolvedGateway, DiscoveryError> {
        let directory = tempdir()?;
        let path = directory.path().join("gateway.json");
        fs::write(&path, r#"{"port":1340,"host":"0.0.0.0","token":"file-secret"}"#)?;
        discover_gateway(&DiscoverOptions {
            url: Some("https://remote.example:443/path".to_owned()),
            token: Some("explicit-secret".to_owned()),
            discovery_path: Some(path),
            ..DiscoverOptions::default()
        })
    }
}
