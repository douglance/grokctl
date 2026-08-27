//! One-time import from the running Grok Bot local-exec daemon.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use thiserror::Error;
use zeroize::Zeroizing;

const FILE_KEY_ENV: &str = "SAND_LOCAL_EXEC_FILE_KEY";
const ENVELOPE_FIELD: &str = "sandSealedFile";
const NONCE_BYTES: usize = 12;
const TAG_BYTES: usize = 16;

/// Secret-safe result of importing a live daemon connection.
#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedConnection {
    /// Temporary discovery file written with mode 0600.
    pub path: String,
    /// Imported gateway origin.
    pub gateway_url: String,
    /// Whether a bearer token was imported.
    pub has_token: bool,
    /// Secret header names, without values.
    pub header_names: Vec<String>,
}

/// Failure importing the running daemon connection.
#[derive(Debug, Error)]
pub enum DaemonImportError {
    /// Required local path is unavailable.
    #[error("the Grok Bot daemon path is unavailable")]
    Path,
    /// A daemon file could not be read or written.
    #[error("daemon connection file operation failed: {0}")]
    Io(#[from] std::io::Error),
    /// A daemon file was invalid.
    #[error("daemon connection data is invalid: {0}")]
    Json(#[from] serde_json::Error),
    /// The recorded daemon process no longer exists or has the wrong identity.
    #[error("the recorded Grok Bot local-exec daemon is not running")]
    Process,
    /// The daemon did not retain its one-time file-key handoff.
    #[error("the running daemon did not retain its connection file key")]
    Key,
    /// The retained key did not match the daemon discovery witness.
    #[error("the daemon connection file key did not match its witness")]
    KeyWitness,
    /// Authenticated decryption failed.
    #[error("the daemon connection file could not be authenticated")]
    Decrypt,
    /// Destination already exists.
    #[error("refusing to overwrite existing connection file: {0}")]
    Exists(String),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DaemonDiscovery {
    pid: u32,
    files_key_id: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionFile {
    url: String,
    token: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
}

/// Import the running desktop daemon connection into a new mode-0600 file.
///
/// # Errors
///
/// Returns [`DaemonImportError`] when daemon identity, key recovery, authenticated
/// decryption, validation, or exclusive file creation fails.
pub fn import_running_daemon(output: &Path) -> Result<ImportedConnection, DaemonImportError> {
    if output.exists() {
        return Err(DaemonImportError::Exists(output.display().to_string()));
    }
    let root = dirs::home_dir().ok_or(DaemonImportError::Path)?.join(".grokbot");
    let discovery = read_discovery(&root.join("local-exec-daemon.json"))?;
    let key = daemon_file_key(&discovery)?;
    verify_key(&key, &discovery.files_key_id)?;
    let sealed = fs::read(root.join("local-exec-daemon-connection.json"))?;
    let plaintext = unseal(&sealed, &key)?;
    let connection = parse_connection(&plaintext)?;
    write_connection(output, &connection)?;
    Ok(imported(output, &connection))
}

fn read_discovery(path: &Path) -> Result<DaemonDiscovery, DaemonImportError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn daemon_file_key(discovery: &DaemonDiscovery) -> Result<Zeroizing<Vec<u8>>, DaemonImportError> {
    let pid = Pid::from_u32(discovery.pid);
    let refresh =
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always).with_environ(UpdateKind::Always);
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, refresh);
    let process = system.process(pid).ok_or(DaemonImportError::Process)?;
    let is_daemon =
        process.cmd().iter().any(|value| value.to_string_lossy().contains("local-exec-daemon"));
    if !is_daemon {
        return Err(DaemonImportError::Process);
    }
    let encoded =
        environment_value(process.environ(), FILE_KEY_ENV).ok_or(DaemonImportError::Key)?;
    let decoded = STANDARD.decode(encoded.as_bytes()).map_err(|_| DaemonImportError::Key)?;
    (decoded.len() == 32).then(|| Zeroizing::new(decoded)).ok_or(DaemonImportError::Key)
}

fn environment_value(values: &[std::ffi::OsString], name: &str) -> Option<String> {
    values.iter().find_map(|entry| {
        let text = entry.to_string_lossy();
        text.strip_prefix(name)?.strip_prefix('=').map(str::to_owned)
    })
}

fn verify_key(key: &[u8], expected: &str) -> Result<(), DaemonImportError> {
    let observed = hex::encode(Sha256::digest(key));
    observed.starts_with(expected).then_some(()).ok_or(DaemonImportError::KeyWitness)
}

fn unseal(bytes: &[u8], key: &[u8]) -> Result<Zeroizing<Vec<u8>>, DaemonImportError> {
    let envelope = serde_json::from_slice::<Value>(bytes)?;
    if envelope[ENVELOPE_FIELD] != 1 {
        return Err(DaemonImportError::Decrypt);
    }
    let encoded = envelope["data"].as_str().ok_or(DaemonImportError::Decrypt)?;
    let payload = STANDARD.decode(encoded).map_err(|_| DaemonImportError::Decrypt)?;
    let nonce_bytes = payload.get(..NONCE_BYTES).ok_or(DaemonImportError::Decrypt)?;
    let tag =
        payload.get(NONCE_BYTES..NONCE_BYTES + TAG_BYTES).ok_or(DaemonImportError::Decrypt)?;
    let ciphertext = payload.get(NONCE_BYTES + TAG_BYTES..).ok_or(DaemonImportError::Decrypt)?;
    let mut combined = Zeroizing::new(Vec::with_capacity(ciphertext.len() + TAG_BYTES));
    combined.extend_from_slice(ciphertext);
    combined.extend_from_slice(tag);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| DaemonImportError::Decrypt)?;
    let nonce = Nonce::try_from(nonce_bytes).map_err(|_| DaemonImportError::Decrypt)?;
    let plaintext =
        cipher.decrypt(&nonce, combined.as_slice()).map_err(|_| DaemonImportError::Decrypt)?;
    Ok(Zeroizing::new(plaintext))
}

fn parse_connection(bytes: &[u8]) -> Result<ConnectionFile, DaemonImportError> {
    let value = serde_json::from_slice::<Value>(bytes)?;
    let url = value["baseUrl"].as_str().ok_or(DaemonImportError::Decrypt)?.to_owned();
    let token = value["token"].as_str().map(str::to_owned).filter(|value| !value.is_empty());
    let headers = serde_json::from_value(value["headers"].clone()).unwrap_or_default();
    Ok(ConnectionFile { url, token, headers })
}

fn write_connection(path: &Path, connection: &ConnectionFile) -> Result<(), DaemonImportError> {
    let parent = path.parent().ok_or(DaemonImportError::Path)?;
    fs::create_dir_all(parent)?;
    let mut file = OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)?;
    file.write_all(&serde_json::to_vec(connection)?)?;
    file.sync_all()?;
    Ok(())
}

fn imported(path: &Path, connection: &ConnectionFile) -> ImportedConnection {
    let mut header_names = connection.headers.keys().cloned().collect::<Vec<_>>();
    header_names.sort();
    ImportedConnection {
        path: path.display().to_string(),
        gateway_url: connection.url.clone(),
        has_token: connection.token.is_some(),
        header_names,
    }
}
