//! Host manifest extraction, validation, and compatibility policy.

mod extract;
mod format;
mod markers;
mod policy;
mod seed;
mod validate;

pub use extract::{ExtractError, extract_host_manifest};
pub use format::{HostManifest, ManifestNotes};
pub use policy::{CommandEffect, CommandPolicy, PolicyError, classify_command};
pub use seed::seed_manifest;
pub use validate::{ManifestError, validate_manifest};
