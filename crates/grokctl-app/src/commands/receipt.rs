//! Local idempotency receipt lookup.

use grokctl_core::receipt::MutationReceipt;
use incurs::cli::Cli;
use incurs::command::{CommandDef, McpAnnotations, McpCommandOptions, TypedResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::open_journal;

#[derive(Clone, Debug, Deserialize, incurs::Args)]
struct GetArgs {
    /// Exact normalized gateway origin.
    gateway_id: String,
    /// Idempotency key.
    key: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
struct GetOutput {
    receipt: Option<MutationReceipt>,
}

pub fn group() -> Cli {
    Cli::create("receipt")
        .description("Inspect local mutation receipts")
        .command("get", get_command())
}

fn get_command() -> CommandDef {
    CommandDef::typed::<GetArgs, (), (), GetOutput, _, _>("get", |ctx| async move {
        let journal = match open_journal() {
            Ok(value) => value,
            Err(error) => return TypedResult::error("JOURNAL_ERROR", error.to_string()),
        };
        match journal.get(&ctx.args.gateway_id, &ctx.args.key) {
            Ok(receipt) => TypedResult::ok(GetOutput { receipt }),
            Err(error) => TypedResult::error("JOURNAL_ERROR", error.to_string()),
        }
    })
    .description("Read one secret-free local receipt")
    .mcp(read_only_mcp())
    .done()
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
