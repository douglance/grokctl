//! Raw gateway and health commands.

use std::time::Duration;

use base64::Engine;
use grokctl_core::domain::{GatewayEvent, HealthResponse};
use grokctl_manifest::{CommandEffect, CommandPolicy};
use incurs::cli::Cli;
use incurs::command::{CommandDef, McpResultContent, TypedResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::{CallOptions, CallOutput, RawCallArgs, gateway_client, mcp_options, raw_call};

pub fn group() -> Cli {
    Cli::create("gateway")
        .description("Inspect or call the Sand gateway")
        .command("health", health_command())
        .command("events", events_command())
        .command("avatar", avatar_command())
        .command("call", call_command())
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct EventsArgs {
    /// Optional event channel filters.
    channels: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, incurs::Options)]
struct EventsOptions {
    /// Gateway origin.
    gateway_url: Option<String>,
    /// Bearer token.
    gateway_token: Option<String>,
    /// Explicit gateway.json path.
    discovery_path: Option<String>,
    /// Permit plaintext remote HTTP.
    #[serde(default)]
    allow_insecure_http: bool,
    /// Maximum events to collect.
    #[serde(default = "default_event_limit")]
    limit: usize,
    /// Maximum collection duration in seconds.
    #[serde(default = "default_event_timeout")]
    timeout_seconds: u64,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct AvatarArgs {
    /// Bot identifier.
    bot_id: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct AvatarOutput {
    data: String,
    mime_type: String,
    etag: Option<String>,
}

fn health_command() -> CommandDef {
    let policy =
        CommandPolicy { name: "health".to_owned(), effect: CommandEffect::Read, open_world: false };
    CommandDef::typed::<(), CallOptions, (), HealthResponse, _, _>("health", |ctx| async move {
        match gateway_client(&ctx.options) {
            Ok(client) => match client.health().await {
                Ok(value) => TypedResult::ok(value),
                Err(error) => TypedResult::error("GATEWAY_ERROR", error.to_string()),
            },
            Err(error) => TypedResult::error("CONFIG_ERROR", error.to_string()),
        }
    })
    .description("Read unauthenticated gateway health")
    .mcp(mcp_options(&policy, "Read unauthenticated gateway health"))
    .done()
}

fn call_command() -> CommandDef {
    let policy = CommandPolicy {
        name: "rawGatewayCall".to_owned(),
        effect: CommandEffect::Destructive,
        open_world: true,
    };
    CommandDef::typed::<RawCallArgs, CallOptions, (), CallOutput, _, _>("call", raw_call)
        .description("Call an exact host command with conservative safety guards")
        .mcp(mcp_options(&policy, "Call an exact host command"))
        .destructive(true)
        .done()
}

fn events_command() -> CommandDef {
    let policy = read_policy("events");
    CommandDef::typed::<EventsArgs, EventsOptions, (), Vec<GatewayEvent>, _, _>(
        "events",
        |ctx| async move {
            let call = event_call_options(&ctx.options);
            let client = match gateway_client(&call) {
                Ok(value) => value,
                Err(error) => return TypedResult::error("CONFIG_ERROR", error.to_string()),
            };
            let duration = Duration::from_secs(ctx.options.timeout_seconds);
            match client.collect_events(&ctx.args.channels, ctx.options.limit, duration).await {
                Ok(events) => TypedResult::ok(events),
                Err(error) => TypedResult::error("EVENT_ERROR", error.to_string()),
            }
        },
    )
    .description("Collect a bounded set of authenticated gateway events")
    .mcp(mcp_options(&policy, "Collect a bounded set of gateway events"))
    .done()
}

fn avatar_command() -> CommandDef {
    let policy = read_policy("avatar");
    let mut mcp = mcp_options(&policy, "Retrieve a Bot avatar");
    mcp.result_content.push(McpResultContent::Image {
        data_pointer: "/data".to_owned(),
        mime_type_pointer: "/mimeType".to_owned(),
    });
    CommandDef::typed::<AvatarArgs, CallOptions, (), AvatarOutput, _, _>(
        "avatar",
        |ctx| async move {
            let client = match gateway_client(&ctx.options) {
                Ok(value) => value,
                Err(error) => return TypedResult::error("CONFIG_ERROR", error.to_string()),
            };
            match client.avatar(&ctx.args.bot_id).await {
                Ok(image) => TypedResult::ok(AvatarOutput {
                    data: base64::engine::general_purpose::STANDARD.encode(image.bytes),
                    mime_type: image.mime_type,
                    etag: image.etag,
                }),
                Err(error) => TypedResult::error("AVATAR_ERROR", error.to_string()),
            }
        },
    )
    .description("Retrieve a Bot avatar")
    .mcp(mcp)
    .done()
}

fn event_call_options(options: &EventsOptions) -> CallOptions {
    CallOptions {
        gateway_url: options.gateway_url.clone(),
        gateway_token: options.gateway_token.clone(),
        discovery_path: options.discovery_path.clone(),
        allow_insecure_http: options.allow_insecure_http,
        ..CallOptions::default()
    }
}

fn read_policy(name: &str) -> CommandPolicy {
    CommandPolicy { name: name.to_owned(), effect: CommandEffect::Read, open_world: false }
}

const fn default_event_limit() -> usize {
    10
}

const fn default_event_timeout() -> u64 {
    30
}
