//! Gateway URL parsing and normalization.

use thiserror::Error;
use url::Url;

/// Invalid gateway URL.
#[derive(Debug, Error)]
pub enum GatewayUrlError {
    /// URL parsing failed.
    #[error("invalid gateway URL: {0}")]
    Parse(#[from] url::ParseError),
    /// Scheme is not HTTP or HTTPS.
    #[error("gateway URL must use http or https")]
    Scheme,
    /// URL contains a username or password.
    #[error("gateway URL must not contain user information")]
    UserInfo,
    /// Plain HTTP to a non-loopback host was not explicitly trusted.
    #[error("non-loopback HTTP requires allow_insecure_http")]
    InsecureRemote,
}

/// Normalize a gateway origin and reject unsafe forms.
///
/// # Errors
///
/// Returns [`GatewayUrlError`] for malformed or untrusted origins.
pub fn normalize_gateway_url(raw: &str, allow_insecure_http: bool) -> Result<Url, GatewayUrlError> {
    let mut url = Url::parse(raw)?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(GatewayUrlError::Scheme);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(GatewayUrlError::UserInfo);
    }
    let host = url.host_str().ok_or(GatewayUrlError::Parse(url::ParseError::EmptyHost))?;
    if is_wildcard(host) {
        url.set_host(Some("127.0.0.1"))?;
    }
    let loopback = url.host_str().is_some_and(is_loopback);
    if url.scheme() == "http" && !loopback && !allow_insecure_http {
        return Err(GatewayUrlError::InsecureRemote);
    }
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}

fn is_wildcard(host: &str) -> bool {
    matches!(host, "0.0.0.0" | "::" | "[::]")
}

fn is_loopback(host: &str) -> bool {
    matches!(host.to_ascii_lowercase().as_str(), "127.0.0.1" | "localhost" | "::1" | "[::1]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_wildcard_and_drops_path() {
        let result = normalize_gateway_url("http://0.0.0.0:1340/api/", false);
        assert!(result.is_ok(), "loopback rewrite must be accepted: {result:?}");
        let Some(url) = result.ok() else { return };
        assert_eq!(url.as_str(), "http://127.0.0.1:1340/");
    }

    #[test]
    fn rejects_untrusted_remote_http() {
        let error = normalize_gateway_url("http://grok.example:1340", false).err();
        assert!(matches!(error, Some(GatewayUrlError::InsecureRemote)));
    }
}
