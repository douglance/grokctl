//! Authenticated avatar retrieval.

use super::{GatewayClient, GatewayError};

/// Binary avatar response with safe content metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AvatarImage {
    /// Response media type.
    pub mime_type: String,
    /// Raw image bytes.
    pub bytes: Vec<u8>,
    /// Entity tag supplied by the host.
    pub etag: Option<String>,
}

impl GatewayClient {
    /// Retrieve a Bot avatar through the authenticated avatar route.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] for missing auth, transport, status, or body failures.
    pub async fn avatar(&self, bot_id: &str) -> Result<AvatarImage, GatewayError> {
        let encoded = url::form_urlencoded::byte_serialize(bot_id.as_bytes()).collect::<String>();
        let (response, request_id) =
            self.protected_get(&format!("/avatars/{encoded}"), "avatar", "image/*").await?;
        let mime_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_owned();
        let etag =
            response.headers().get("etag").and_then(|value| value.to_str().ok()).map(str::to_owned);
        let bytes = response.bytes().await.map_err(|error| GatewayError::Transport {
            command: "avatar".to_owned(),
            request_id,
            message: error.to_string(),
        })?;
        Ok(AvatarImage { mime_type, bytes: bytes.to_vec(), etag })
    }
}
