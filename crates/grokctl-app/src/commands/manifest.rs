//! Compatibility manifest and live drift verdicts.

use grokctl_core::compat::{CompatVerdict, evaluate_compat};
use grokctl_core::domain::HostStatus;
use grokctl_manifest::{HostManifest, seed_manifest};
use incurs::cli::Cli;
use incurs::command::{CommandDef, TypedResult};

use super::common::{CallOptions, gateway_client, mcp_options};

pub fn group() -> Cli {
    Cli::create("manifest")
        .description("Inspect compatibility evidence and live drift")
        .command("show", show_command())
        .command("check", check_command())
}

fn show_command() -> CommandDef {
    let policy = grokctl_manifest::classify_command("getHostStatus");
    CommandDef::typed::<(), (), (), HostManifest, _, _>("show", |_| async move {
        TypedResult::ok(seed_manifest())
    })
    .description("Show the pinned compatibility seed and provenance")
    .mcp(mcp_options(&policy, "Show the pinned compatibility seed"))
    .done()
}

fn check_command() -> CommandDef {
    let policy = grokctl_manifest::classify_command("getHostStatus");
    CommandDef::typed::<(), CallOptions, (), CompatVerdict, _, _>("check", |ctx| async move {
        let client = match gateway_client(&ctx.options) {
            Ok(value) => value,
            Err(error) => return TypedResult::error("CONFIG_ERROR", error.to_string()),
        };
        match client.command_typed::<HostStatus>("getHostStatus", None).await {
            Ok(live) => TypedResult::ok(evaluate_compat(&seed_manifest(), &live)),
            Err(error) => TypedResult::error("GATEWAY_ERROR", error.to_string()),
        }
    })
    .description("Compare live host status and warn without blocking calls")
    .mcp(mcp_options(&policy, "Compare live host status with the pinned manifest"))
    .done()
}
