//! Secret-safe bearer token wrapper.

use secrecy::{ExposeSecret, SecretString};

/// Bearer token that never exposes its value through `Debug` or serialization.
#[derive(Clone)]
pub struct GatewaySecret(SecretString);

impl GatewaySecret {
    /// Construct a secret from a runtime value.
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(SecretString::from(value))
    }

    /// Expose the token only at the HTTP authorization boundary.
    #[must_use]
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl std::fmt::Debug for GatewaySecret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("GatewaySecret([redacted])")
    }
}
