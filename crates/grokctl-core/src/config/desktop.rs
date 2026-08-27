//! Grok Bot desktop gateway descriptor parsing.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use thiserror::Error;

use super::ImportedConnection;
use super::electron_crypto::decrypt_v10;

const KEYCHAIN_SERVICE: &str = "Grok Bot Safe Storage";
const KEYCHAIN_ACCOUNT: &str = "Grok Bot Key";

pub(super) struct DesktopGateway {
    pub(super) connection: DesktopConnection,
    pub(super) path: PathBuf,
}

#[derive(Debug, Error)]
pub(super) enum DesktopError {
    #[error("could not read the desktop gateway descriptor: {0}")]
    Read(#[from] std::io::Error),
    #[error("desktop gateway descriptor is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("desktop gateway key is unavailable")]
    Keychain,
    #[error("desktop gateway descriptor could not be decrypted")]
    Decrypt,
}

/// Failure importing a desktop connection into a temporary discovery file.
#[derive(Debug, Error)]
pub enum DesktopImportError {
    /// Desktop descriptor access or decryption failed.
    #[error("{0}")]
    Desktop(String),
    /// Destination file access failed.
    #[error("desktop connection file operation failed: {0}")]
    Io(#[from] std::io::Error),
    /// Destination JSON encoding failed.
    #[error("desktop connection data is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// Destination already exists.
    #[error("refusing to overwrite existing connection file: {0}")]
    Exists(String),
}

#[derive(Debug, Deserialize)]
struct DesktopDescriptor {
    version: u8,
    entries: HashMap<String, EncryptedEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptedEntry {
    saved_at_ms: u64,
    encrypted: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesktopConnection {
    pub(super) base_url: String,
    pub(super) token: Option<String>,
    #[serde(default)]
    pub(super) headers: HashMap<String, String>,
}

fn parse_descriptor(
    bytes: &[u8],
    decrypt: impl Fn(&str) -> Option<String>,
) -> Result<Option<DesktopConnection>, serde_json::Error> {
    let mut descriptor = serde_json::from_slice::<DesktopDescriptor>(bytes)?;
    if !matches!(descriptor.version, 1 | 2) {
        return Ok(None);
    }
    let mut entries = descriptor.entries.drain().map(|(_, value)| value).collect::<Vec<_>>();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.saved_at_ms));
    Ok(entries.into_iter().find_map(|entry| {
        let plaintext = decrypt(&entry.encrypted)?;
        serde_json::from_str::<DesktopConnection>(&plaintext).ok()
    }))
}

#[cfg(target_os = "macos")]
pub(super) fn discover_desktop() -> Result<Option<DesktopGateway>, DesktopError> {
    let Some(path) = descriptor_path() else { return Ok(None) };
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    let password =
        security_framework::passwords::get_generic_password(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
            .map_err(|_| DesktopError::Keychain)?;
    let connection = parse_descriptor(&bytes, |encrypted| decrypt(encrypted, &password))?
        .ok_or(DesktopError::Decrypt)?;
    Ok(Some(DesktopGateway { connection, path }))
}

/// Import the desktop descriptor using password bytes supplied through a secret channel.
///
/// # Errors
///
/// Returns [`DesktopImportError`] when the descriptor cannot be read or decrypted,
/// or when the mode-0600 destination cannot be created exclusively.
pub fn import_desktop_with_password(
    output: &Path,
    password: &[u8],
) -> Result<ImportedConnection, DesktopImportError> {
    if output.exists() {
        return Err(DesktopImportError::Exists(output.display().to_string()));
    }
    let path = descriptor_path()
        .ok_or(DesktopError::Decrypt)
        .map_err(|error| DesktopImportError::Desktop(error.to_string()))?;
    let bytes = fs::read(path)?;
    import_descriptor(&bytes, output, password)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn discover_desktop() -> Result<Option<DesktopGateway>, DesktopError> {
    Ok(None)
}

fn descriptor_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("Grok Bot").join("gateway-descriptor.json"))
}

fn decrypt(encrypted: &str, password: &[u8]) -> Option<String> {
    let ciphertext = STANDARD.decode(encrypted).ok()?;
    decrypt_v10(&ciphertext, password)
}

fn import_descriptor(
    bytes: &[u8],
    output: &Path,
    password: &[u8],
) -> Result<ImportedConnection, DesktopImportError> {
    let connection = parse_descriptor(bytes, |encrypted| decrypt(encrypted, password))?
        .ok_or(DesktopError::Decrypt)
        .map_err(|error| DesktopImportError::Desktop(error.to_string()))?;
    write_connection(output, &connection)?;
    Ok(imported_connection(output, &connection))
}

fn write_connection(
    output: &Path,
    connection: &DesktopConnection,
) -> Result<(), DesktopImportError> {
    let parent = output
        .parent()
        .ok_or(DesktopError::Decrypt)
        .map_err(|error| DesktopImportError::Desktop(error.to_string()))?;
    fs::create_dir_all(parent)?;
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(output)?;
    let value = serde_json::json!({
        "url": connection.base_url,
        "token": connection.token,
        "headers": connection.headers,
    });
    file.write_all(&serde_json::to_vec(&value)?)?;
    file.sync_all()?;
    Ok(())
}

fn imported_connection(output: &Path, connection: &DesktopConnection) -> ImportedConnection {
    let mut header_names = connection.headers.keys().cloned().collect::<Vec<_>>();
    header_names.sort();
    ImportedConnection {
        path: output.display().to_string(),
        gateway_url: connection.base_url.clone(),
        has_token: connection.token.as_ref().is_some_and(|token| !token.is_empty()),
        header_names,
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn newest_decryptable_entry_preserves_gateway_headers() {
        let bytes = br#"{
          "version": 2,
          "entries": {
            "older": {"savedAtMs": 10, "encrypted": "old"},
            "newer": {"savedAtMs": 20, "encrypted": "new"}
          }
        }"#;

        let result = parse_descriptor(bytes, |value| {
            match value {
            "new" => Some(r#"{"baseUrl":"https://box.example","token":"bearer","headers":{"x-sand-network-token":"network"}}"#.to_owned()),
            _ => None,
        }
        });

        assert!(result.is_ok(), "fixture should parse: {result:?}");
        let connection = result.ok().flatten();
        assert_eq!(
            connection.as_ref().map(|row| row.base_url.as_str()),
            Some("https://box.example")
        );
        assert_eq!(
            connection.and_then(|row| row.headers.get("x-sand-network-token").cloned()),
            Some("network".to_owned())
        );
    }

    #[test]
    fn ignores_unknown_descriptor_versions() {
        let bytes = br#"{"version":3,"entries":{}}"#;

        let result = parse_descriptor(bytes, |_| None);

        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn imports_v2_descriptor_to_mode_0600_file() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let output = directory.path().join("gateway.json");
        let bytes = br#"{
          "version": 2,
          "entries": {
            "fixture": {
              "savedAtMs": 20,
              "encrypted": "djEwwqTpnXPs2YcGDFMu2jc4J/m1BZfznPWMWgGxm2+x/PGpLxWCKti1bc3mY5oZq9/5"
            }
          }
        }"#;

        let result = import_descriptor(bytes, &output, b"peanuts")?;

        assert_eq!(result.gateway_url, "http://127.0.0.1:1340");
        let contents = fs::read_to_string(&output)?;
        assert!(contents.contains("http://127.0.0.1:1340"));
        let mode = fs::metadata(output)?.permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        Ok(())
    }
}
