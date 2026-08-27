//! Gateway and profile configuration.

mod daemon;
mod desktop;
mod discovery;
mod electron_crypto;
mod profile;
mod secret;
mod url;

pub use daemon::{DaemonImportError, ImportedConnection, import_running_daemon};
pub use desktop::{DesktopImportError, import_desktop_with_password};
pub use discovery::{
    DiscoverOptions, DiscoveryError, DiscoveryFile, ResolvedGateway, discover_gateway,
};
pub use profile::{Profile, Profiles};
pub use secret::GatewaySecret;
pub use url::{GatewayUrlError, normalize_gateway_url};
