//! Public Bot template commands.

use grokctl_manifest::{CommandEffect, classify_command};
use incurs::cli::Cli;
use incurs::command::{CommandDef, TypedResult};
use serde::Deserialize;
use serde_json::{Value, json};

use super::common::{CallOptions, CallOutput, call_host_empty, call_host_json, mcp_options};

#[derive(Clone, Debug, Default, Deserialize, incurs::Options)]
struct TemplateOptions {
    /// Gateway origin. Environment and gateway.json discovery are fallbacks.
    gateway_url: Option<String>,
    /// Bearer token. Prefer `GROKCTL_GATEWAY_TOKEN` outside interactive use.
    gateway_token: Option<String>,
    /// Explicit gateway.json path.
    discovery_path: Option<String>,
    /// Permit plaintext HTTP to a remote host.
    #[serde(default)]
    allow_insecure_http: bool,
    /// Stable key required by every mutation.
    idempotency_key: Option<String>,
    /// Admit public, visibility-changing, or destructive operations.
    #[serde(default)]
    unsafe_mode: bool,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct ShareArgs {
    /// Public template share ID.
    share_id: String,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct VersionArgs {
    /// Public template share ID.
    share_id: String,
    /// Template version.
    version: u64,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct SourceBotArgs {
    /// Source Bot ID.
    source_bot_id: String,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct VisibilityArgs {
    /// Public template share ID.
    share_id: String,
    /// Template visibility: public or team.
    visibility: String,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct AddArgs {
    /// Public template share ID.
    share_id: String,
    /// Stable ID for the new Bot.
    bot_id: String,
    /// Name for the new Bot.
    name: String,
    /// Avatar shape from the template preview.
    avatar_shape: String,
    /// Avatar color from the template preview.
    avatar_color: String,
    /// Active template version shown in the preview.
    expected_active_version: u64,
}

pub fn group() -> Cli {
    Cli::create("template")
        .description("Manage public Bot templates")
        .command("list", list_command())
        .command("version", version_command())
        .command("for-bot", source_bot_command())
        .command("add", add_command())
        .command("publish", publish_command())
        .command("visibility", visibility_command())
        .command("delete", delete_command())
}

fn list_command() -> CommandDef {
    let policy = classify_command("listBotTemplates");
    CommandDef::typed::<(), TemplateOptions, (), CallOutput, _, _>("list", |ctx| async move {
        call_host_empty("listBotTemplates", &call_options(&ctx.options)).await
    })
    .description("List owned Bot templates")
    .mcp(mcp_options(&policy, "List owned Bot templates"))
    .done()
}

fn version_command() -> CommandDef {
    command::<VersionArgs>(
        "version",
        "getBotTemplateVersion",
        "Read one Bot template version",
        |args| publish_body(&args.share_id, args.version),
    )
}

fn source_bot_command() -> CommandDef {
    command::<SourceBotArgs>(
        "for-bot",
        "getBotTemplateForSourceAgent",
        "Read the template created from one Bot",
        |args| source_agent_body(&args.source_bot_id),
    )
}

fn add_command() -> CommandDef {
    command::<AddArgs>(
        "add",
        "createAgentFromTemplate",
        "Add a Bot from a public template",
        add_body,
    )
}

fn publish_command() -> CommandDef {
    command::<VersionArgs>(
        "publish",
        "publishBotTemplate",
        "Publish a Bot template version",
        |args| publish_body(&args.share_id, args.version),
    )
}

fn visibility_command() -> CommandDef {
    let policy = classify_command("setBotTemplateVisibility");
    CommandDef::typed::<VisibilityArgs, TemplateOptions, (), CallOutput, _, _>(
        "visibility",
        |ctx| async move {
            if !valid_visibility(&ctx.args.visibility) {
                return TypedResult::error(
                    "INVALID_VISIBILITY",
                    "visibility must be public or team",
                );
            }
            call_host_json(
                "setBotTemplateVisibility",
                &call_options(&ctx.options),
                visibility_body(&ctx.args.share_id, &ctx.args.visibility),
            )
            .await
        },
    )
    .description("Set a Bot template's audience")
    .mcp(mcp_options(&policy, "Set a Bot template's audience"))
    .destructive(true)
    .done()
}

fn delete_command() -> CommandDef {
    command::<ShareArgs>("delete", "deleteBotTemplate", "Delete a shared Bot template", |args| {
        share_body(&args.share_id)
    })
}

fn command<A>(
    cli_name: &'static str,
    host_name: &'static str,
    description: &'static str,
    body: fn(&A) -> Value,
) -> CommandDef
where
    A: incurs::schema::IncurSchema + Send + Sync + 'static,
{
    let policy = classify_command(host_name);
    let destructive = policy.effect == CommandEffect::Destructive;
    CommandDef::typed::<A, TemplateOptions, (), CallOutput, _, _>(cli_name, move |ctx| async move {
        call_host_json(host_name, &call_options(&ctx.options), body(&ctx.args)).await
    })
    .description(description)
    .mcp(mcp_options(&policy, description))
    .destructive(destructive)
    .done()
}

fn call_options(options: &TemplateOptions) -> CallOptions {
    CallOptions {
        gateway_url: options.gateway_url.clone(),
        gateway_token: options.gateway_token.clone(),
        discovery_path: options.discovery_path.clone(),
        allow_insecure_http: options.allow_insecure_http,
        idempotency_key: options.idempotency_key.clone(),
        unsafe_mode: options.unsafe_mode,
        ..CallOptions::default()
    }
}

fn share_body(share_id: &str) -> Value {
    json!({ "shareId": share_id })
}

fn add_body(args: &AddArgs) -> Value {
    json!({
        "shareId": args.share_id,
        "agentId": args.bot_id,
        "name": args.name,
        "avatarShape": args.avatar_shape,
        "avatarColor": args.avatar_color,
        "expectedActiveVersion": args.expected_active_version,
    })
}

fn publish_body(share_id: &str, version: u64) -> Value {
    json!({ "shareId": share_id, "version": version })
}

fn source_agent_body(source_agent_id: &str) -> Value {
    json!({ "sourceAgentId": source_agent_id })
}

fn visibility_body(share_id: &str, visibility: &str) -> Value {
    json!({ "shareId": share_id, "visibility": visibility })
}

fn valid_visibility(value: &str) -> bool {
    matches!(value, "public" | "team")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn template_payloads_match_the_observed_gateway_contract() {
        let add = AddArgs {
            share_id: "share-1".to_owned(),
            bot_id: "bot-1".to_owned(),
            name: "Release Captain".to_owned(),
            avatar_shape: "circle".to_owned(),
            avatar_color: "blue".to_owned(),
            expected_active_version: 3,
        };

        assert_eq!(
            add_body(&add),
            json!({
                "shareId": "share-1",
                "agentId": "bot-1",
                "name": "Release Captain",
                "avatarShape": "circle",
                "avatarColor": "blue",
                "expectedActiveVersion": 3,
            })
        );
        assert_eq!(share_body("share-1"), json!({ "shareId": "share-1" }));
        assert_eq!(publish_body("share-1", 3), json!({ "shareId": "share-1", "version": 3 }));
        assert_eq!(source_agent_body("bot-1"), json!({ "sourceAgentId": "bot-1" }));
        assert_eq!(
            visibility_body("share-1", "public"),
            json!({ "shareId": "share-1", "visibility": "public" })
        );
        assert!(valid_visibility("public"));
        assert!(valid_visibility("team"));
        assert!(!valid_visibility("private"));
    }
}
