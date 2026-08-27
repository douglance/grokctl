//! Prompt acceptance and wait workflow.

use std::time::{Duration, Instant};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::time::sleep;

use crate::domain::{BotSummary, PromptAccepted};
use crate::gateway::GatewayError;

use super::{BotService, ResolveBotError};

/// Terminal client-side prompt status.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptStatus {
    /// Host accepted the prompt and the caller did not wait.
    Accepted,
    /// Bot and its background work are idle.
    Idle,
    /// Bot awaits a user widget or input response.
    AwaitingUser,
    /// Wait deadline elapsed.
    Timeout,
}

/// Prompt receipt returned by CLI and MCP.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptResult {
    /// Resolved Bot identifier.
    pub bot_id: String,
    /// Whether the host accepted the prompt.
    pub accepted: bool,
    /// Client-side status.
    pub status: PromptStatus,
    /// Elapsed milliseconds.
    pub elapsed_ms: u64,
    /// Last assistant text, when requested and available.
    pub reply: Option<String>,
}

/// Wait behavior for an accepted prompt.
#[derive(Clone, Debug)]
pub struct PromptWaitOptions {
    /// Wait after acceptance.
    pub wait: bool,
    /// Maximum wait duration.
    pub timeout: Duration,
    /// Poll interval.
    pub interval: Duration,
    /// Read the transcript tail after waiting.
    pub include_reply: bool,
}

enum WaitStep {
    Done(PromptStatus),
    Sleep(Duration),
}

impl Default for PromptWaitOptions {
    fn default() -> Self {
        Self {
            wait: false,
            timeout: Duration::from_secs(600),
            interval: Duration::from_secs(1),
            include_reply: true,
        }
    }
}

impl BotService {
    /// Send a prompt and optionally wait for the Bot to become idle.
    ///
    /// # Errors
    ///
    /// Returns [`ResolveBotError`] for resolution or gateway failures.
    pub async fn prompt(
        &self,
        reference: &str,
        prompt: &str,
        client_nonce: &str,
        options: &PromptWaitOptions,
    ) -> Result<PromptResult, ResolveBotError> {
        let started = Instant::now();
        let bot_id = self.resolve(reference).await?;
        let body = json!({ "agentId": bot_id, "prompt": prompt, "clientNonce": client_nonce });
        let accepted: PromptAccepted =
            self.client().command_typed("sendPrompt", Some(&body)).await?;
        if !options.wait {
            return Ok(prompt_result(
                bot_id,
                accepted.accepted,
                PromptStatus::Accepted,
                started,
                None,
            ));
        }
        let status = self.wait_for_idle(&bot_id, client_nonce, options).await?;
        let reply =
            if options.include_reply { self.transcript_reply(&bot_id).await? } else { None };
        Ok(prompt_result(bot_id, accepted.accepted, status, started, reply))
    }

    async fn wait_for_idle(
        &self,
        bot_id: &str,
        nonce: &str,
        options: &PromptWaitOptions,
    ) -> Result<PromptStatus, GatewayError> {
        let started = Instant::now();
        loop {
            match self.wait_step(bot_id, nonce, options, started).await? {
                WaitStep::Done(status) => return Ok(status),
                WaitStep::Sleep(duration) => sleep(duration).await,
            }
        }
    }

    async fn wait_step(
        &self,
        bot_id: &str,
        nonce: &str,
        options: &PromptWaitOptions,
        started: Instant,
    ) -> Result<WaitStep, GatewayError> {
        let status = self.poll_status(bot_id, nonce).await?;
        let elapsed = started.elapsed();
        Ok(match (status, elapsed >= options.timeout) {
            (Some(status), _) => WaitStep::Done(status),
            (None, true) => WaitStep::Done(PromptStatus::Timeout),
            (None, false) => {
                WaitStep::Sleep(options.interval.min(options.timeout.saturating_sub(elapsed)))
            }
        })
    }

    async fn poll_status(
        &self,
        bot_id: &str,
        nonce: &str,
    ) -> Result<Option<PromptStatus>, GatewayError> {
        let rows = self.list().await?;
        let Some(bot) = rows.iter().find(|row| row.id == bot_id) else {
            return Ok(Some(PromptStatus::Idle));
        };
        if self.background_busy(bot_id, nonce, bot).await? {
            return Ok(None);
        }
        Ok(Some(terminal_status(bot)))
    }

    async fn background_busy(
        &self,
        bot_id: &str,
        nonce: &str,
        bot: &BotSummary,
    ) -> Result<bool, GatewayError> {
        let id = json!({ "id": bot_id });
        let tasks: Vec<Value> = self.client().command_typed("getAsyncTasks", Some(&id)).await?;
        let subagents: Vec<Value> = self.client().command_typed("getSubagents", Some(&id)).await?;
        let acceptance: Value = self
            .client()
            .command(
                "promptAcceptanceStatus",
                Some(&json!({ "accountSlot": "host", "clientNonce": nonce })),
            )
            .await?;
        let pending = acceptance_pending(&acceptance);
        let running_subagent = subagents.iter().any(|row| row["status"] == "running");
        Ok(bot.is_running
            || bot.is_composing_message
            || !tasks.is_empty()
            || running_subagent
            || pending)
    }

    async fn transcript_reply(&self, bot_id: &str) -> Result<Option<String>, GatewayError> {
        let value = self
            .client()
            .command("getAgentTranscriptTail", Some(&json!({ "id": bot_id, "limit": 20 })))
            .await?;
        Ok(last_text(&value))
    }
}

fn acceptance_pending(value: &Value) -> bool {
    value["outcome"] == "not-found"
        || (value["outcome"] == "found" && value["record"]["status"] == "pending")
}

const fn terminal_status(bot: &BotSummary) -> PromptStatus {
    if bot.awaiting_user_response.is_some() {
        PromptStatus::AwaitingUser
    } else {
        PromptStatus::Idle
    }
}

fn prompt_result(
    bot_id: String,
    accepted: bool,
    status: PromptStatus,
    started: Instant,
    reply: Option<String>,
) -> PromptResult {
    PromptResult {
        bot_id,
        accepted,
        status,
        elapsed_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
        reply,
    }
}

fn last_text(value: &Value) -> Option<String> {
    let entries = value.get("entries").and_then(Value::as_array).or_else(|| value.as_array())?;
    entries.iter().rev().find_map(|entry| {
        let role = entry.get("role").and_then(Value::as_str).unwrap_or_default();
        let kind = entry.get("type").and_then(Value::as_str).unwrap_or_default();
        if role == "assistant" || kind == "send-message" {
            entry
                .get("text")
                .or_else(|| entry.get("content"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        } else {
            None
        }
    })
}
