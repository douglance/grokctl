//! Secret-safe connection discovery.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use grokctl_core::config::{
    DiscoverOptions, ImportedConnection, discover_gateway, import_desktop_with_password,
    import_running_daemon,
};
use incurs::cli::Cli;
use incurs::command::{CommandDef, McpAnnotations, McpCommandOptions, TypedResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

#[derive(Clone, Debug, Default, Deserialize, incurs::Options)]
struct ShowOptions {
    /// Gateway origin override.
    gateway_url: Option<String>,
    /// Bearer token override.
    gateway_token: Option<String>,
    /// Explicit gateway.json path.
    discovery_path: Option<String>,
    /// Permit plaintext remote HTTP.
    #[serde(default)]
    allow_insecure_http: bool,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct ImportDaemonArgs {
    /// New temporary connection file path.
    output: String,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct ImportDesktopArgs {
    /// New temporary connection file path.
    output: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileOutput {
    gateway_url: String,
    has_token: bool,
    discovery_path: Option<String>,
}

pub fn group() -> Cli {
    Cli::create("profile")
        .description("Resolve gateway connection settings")
        .command("show", show_command())
        .command("import-desktop", import_desktop_command())
        .command("import-daemon", import_daemon_command())
}

fn import_desktop_command() -> CommandDef {
    CommandDef::typed::<ImportDesktopArgs, (), (), ImportedConnection, _, _>(
        "import-desktop",
        |ctx| async move {
            let password = read_password();
            match password.and_then(|value| {
                import_desktop_with_password(&PathBuf::from(ctx.args.output), &value)
                    .map_err(|error| error.to_string())
            }) {
                Ok(result) => TypedResult::ok(result),
                Err(error) => TypedResult::error("DESKTOP_IMPORT_ERROR", error),
            }
        },
    )
    .description("Import desktop credentials from a Keychain password supplied on stdin")
    .done()
}

fn read_password() -> Result<Zeroizing<Vec<u8>>, String> {
    let mut password = Zeroizing::new(Vec::new());
    std::io::stdin()
        .read_to_end(&mut password)
        .map_err(|error| format!("could not read password from stdin: {error}"))?;
    while password.last().is_some_and(|byte| matches!(byte, b'\n' | b'\r')) {
        password.pop();
    }
    (!password.is_empty())
        .then_some(password)
        .ok_or_else(|| "password supplied on stdin was empty".to_owned())
}

fn import_daemon_command() -> CommandDef {
    CommandDef::typed::<ImportDaemonArgs, (), (), ImportedConnection, _, _>(
        "import-daemon",
        |ctx| async move {
            match import_running_daemon(&PathBuf::from(ctx.args.output)) {
                Ok(result) => TypedResult::ok(result),
                Err(error) => TypedResult::error("DAEMON_IMPORT_ERROR", error.to_string()),
            }
        },
    )
    .description("Import a temporary connection from the running Grok Bot daemon")
    .done()
}

fn show_command() -> CommandDef {
    CommandDef::typed::<(), ShowOptions, (), ProfileOutput, _, _>("show", |ctx| async move {
        match discover_gateway(&discover_options(&ctx.options)) {
            Ok(gateway) => TypedResult::ok(ProfileOutput {
                gateway_url: gateway.base_url.to_string(),
                has_token: gateway.has_token,
                discovery_path: gateway.discovery_path.map(|path| path.display().to_string()),
            }),
            Err(error) => TypedResult::error("DISCOVERY_ERROR", error.to_string()),
        }
    })
    .description("Show resolved gateway settings without token bytes")
    .mcp(read_only_mcp())
    .done()
}

fn discover_options(options: &ShowOptions) -> DiscoverOptions {
    DiscoverOptions {
        url: options.gateway_url.clone(),
        token: options.gateway_token.clone(),
        discovery_path: options.discovery_path.as_ref().map(PathBuf::from),
        env: std::env::vars().collect::<HashMap<_, _>>(),
        allow_insecure_http: options.allow_insecure_http,
    }
}

fn read_only_mcp() -> McpCommandOptions {
    McpCommandOptions {
        annotations: Some(McpAnnotations {
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
            ..McpAnnotations::default()
        }),
        ..McpCommandOptions::default()
    }
}
