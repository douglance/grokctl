//! Bot roster operations.

use serde_json::json;
use thiserror::Error;

use crate::domain::BotSummary;
use crate::gateway::{GatewayClient, GatewayError};

/// Bot name or identifier resolution failure.
#[derive(Debug, Error)]
pub enum ResolveBotError {
    /// Gateway access failed.
    #[error(transparent)]
    Gateway(#[from] GatewayError),
    /// No Bot matched the supplied reference.
    #[error("no Bot matches {0}")]
    NotFound(String),
    /// More than one Bot had the same exact name.
    #[error("Bot name {name} is ambiguous; matching IDs: {ids:?}")]
    Ambiguous {
        /// Ambiguous display name.
        name: String,
        /// Candidate identifiers.
        ids: Vec<String>,
    },
}

/// Typed Bot operations over a gateway client.
#[derive(Clone, Debug)]
pub struct BotService {
    client: GatewayClient,
}

impl BotService {
    /// Construct a Bot service.
    #[must_use]
    pub const fn new(client: GatewayClient) -> Self {
        Self { client }
    }

    /// List live roster rows.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] if the host call or response fails.
    pub async fn list(&self) -> Result<Vec<BotSummary>, GatewayError> {
        self.client.command_typed("listAgents", None).await
    }

    /// Resolve a UUID/subagent ID or exact case-insensitive display name.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveBotError`] for gateway failures, no match, or duplicate names.
    pub async fn resolve(&self, reference: &str) -> Result<String, ResolveBotError> {
        let needle = reference.trim();
        let rows = self.list().await?;
        if let Some(row) = rows.iter().find(|row| row.id == needle) {
            return Ok(row.id.clone());
        }
        let matches = rows
            .iter()
            .filter(|row| row.name.eq_ignore_ascii_case(needle))
            .map(|row| row.id.clone())
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [id] => Ok(id.clone()),
            [] => Err(ResolveBotError::NotFound(needle.to_owned())),
            ids => Err(ResolveBotError::Ambiguous { name: needle.to_owned(), ids: ids.to_vec() }),
        }
    }

    /// Delete a Bot by exact identifier or name.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveBotError`] for resolution or gateway failures.
    pub async fn delete(&self, reference: &str) -> Result<serde_json::Value, ResolveBotError> {
        let id = self.resolve(reference).await?;
        Ok(self.client.command("deleteAgent", Some(&json!({ "id": id }))).await?)
    }

    /// Access the underlying typed gateway for workflows.
    #[must_use]
    pub const fn client(&self) -> &GatewayClient {
        &self.client
    }
}
