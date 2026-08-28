//! Bot roster and prompt commands.

use std::time::Duration;

use grokctl_core::domain::BotSummary;
use grokctl_core::service::{BotService, PromptResult, PromptWaitOptions};
use grokctl_manifest::{CommandEffect, CommandPolicy};
use incurs::cli::Cli;
use incurs::command::{CommandDef, TypedResult};
use serde::Deserialize;

use super::common::{CallOptions, fixed_command, gateway_client, mcp_options};

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct PromptArgs {
    /// Bot ID or exact display name.
    bot: String,
    /// Prompt text.
    prompt: String,
}

#[derive(Clone, Debug, Default, Deserialize, incurs::Options)]
struct PromptOptions {
    /// Gateway origin.
    gateway_url: Option<String>,
    /// Bearer token.
    gateway_token: Option<String>,
    /// Explicit gateway.json path.
    discovery_path: Option<String>,
    /// Permit plaintext remote HTTP.
    #[serde(default)]
    allow_insecure_http: bool,
    /// Stable client nonce and idempotency witness.
    idempotency_key: String,
    /// Wait until the Bot stops working.
    #[serde(default)]
    wait: bool,
    /// Maximum wait in seconds.
    #[serde(default = "default_timeout")]
    timeout_seconds: u64,
    /// Poll interval in milliseconds.
    #[serde(default = "default_interval")]
    interval_ms: u64,
    /// Omit transcript reply lookup.
    #[serde(default)]
    no_reply: bool,
}

pub fn group() -> Cli {
    Cli::create("bot")
        .description("List, inspect, and prompt Bots")
        .group(super::template::group())
        .command("list", list_command())
        .command("prompt", prompt_command())
        .command("count", fixed_command("count", "countAgents", "Count Bots"))
        .command("search", fixed_command("search", "searchAgents", "Search Bots"))
        .command("create", fixed_command("create", "createAgent", "Create a Bot"))
        .command("update", fixed_command("update", "updateAgent", "Update a Bot"))
        .command("delete", fixed_command("delete", "deleteAgent", "Delete a Bot"))
        .command("duplicate", fixed_command("duplicate", "duplicateAgent", "Duplicate a Bot"))
        .command("kickstart", fixed_command("kickstart", "kickstartAgent", "Kickstart a Bot"))
        .command(
            "transcript-tail",
            fixed_command("transcript-tail", "getAgentTranscriptTail", "Read a transcript tail"),
        )
}

fn list_command() -> CommandDef {
    let policy = read_policy("listAgents");
    CommandDef::typed::<(), CallOptions, (), Vec<BotSummary>, _, _>("list", |ctx| async move {
        let result = gateway_client(&ctx.options).map(BotService::new);
        match result {
            Ok(service) => match service.list().await {
                Ok(rows) => TypedResult::ok(rows),
                Err(error) => TypedResult::error("GATEWAY_ERROR", error.to_string()),
            },
            Err(error) => TypedResult::error("CONFIG_ERROR", error.to_string()),
        }
    })
    .description("List Bots and groups")
    .mcp(mcp_options(&policy, "List Bots and groups"))
    .done()
}

fn prompt_command() -> CommandDef {
    let policy = CommandPolicy {
        name: "sendPrompt".to_owned(),
        effect: CommandEffect::Mutation,
        open_world: true,
    };
    CommandDef::typed::<PromptArgs, PromptOptions, (), PromptResult, _, _>(
        "prompt",
        |ctx| async move {
            let call = prompt_call_options(&ctx.options);
            match gateway_client(&call) {
                Ok(client) => run_prompt(BotService::new(client), ctx.args, ctx.options).await,
                Err(error) => TypedResult::error("CONFIG_ERROR", error.to_string()),
            }
        },
    )
    .description("Send a prompt and optionally wait for the Bot")
    .mcp(mcp_options(&policy, "Send a prompt and optionally wait for the Bot"))
    .done()
}

async fn run_prompt(
    service: BotService,
    args: PromptArgs,
    options: PromptOptions,
) -> TypedResult<PromptResult> {
    let wait = PromptWaitOptions {
        wait: options.wait,
        timeout: Duration::from_secs(options.timeout_seconds),
        interval: Duration::from_millis(options.interval_ms),
        include_reply: !options.no_reply,
    };
    match service.prompt(&args.bot, &args.prompt, &options.idempotency_key, &wait).await {
        Ok(result) => TypedResult::ok(result),
        Err(error) => TypedResult::error("PROMPT_ERROR", error.to_string()),
    }
}

fn prompt_call_options(options: &PromptOptions) -> CallOptions {
    CallOptions {
        gateway_url: options.gateway_url.clone(),
        gateway_token: options.gateway_token.clone(),
        discovery_path: options.discovery_path.clone(),
        allow_insecure_http: options.allow_insecure_http,
        idempotency_key: Some(options.idempotency_key.clone()),
        ..CallOptions::default()
    }
}

fn read_policy(name: &str) -> CommandPolicy {
    CommandPolicy { name: name.to_owned(), effect: CommandEffect::Read, open_world: false }
}

const fn default_timeout() -> u64 {
    600
}

const fn default_interval() -> u64 {
    1_000
}
