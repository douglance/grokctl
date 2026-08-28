//! Shared gateway command inputs, policy, and execution.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use grokctl_core::config::{DiscoverOptions, discover_gateway};
use grokctl_core::gateway::{GatewayClient, GatewayClientOptions};
use grokctl_core::receipt::{MutationReceipt, ReceiptJournal, ReceiptState, canonical_json_hash};
use grokctl_manifest::{CommandEffect, CommandPolicy, PolicyError, classify_command};
use incurs::cli::Cli;
use incurs::command::{CommandDef, McpAnnotations, McpCommandOptions, TypedContext, TypedResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, Default, Deserialize, incurs::Options)]
pub struct CallOptions {
    /// JSON request body. Defaults to null.
    pub body: Option<String>,
    /// Gateway origin. Environment and gateway.json discovery are fallbacks.
    pub gateway_url: Option<String>,
    /// Bearer token. Prefer `GROKCTL_GATEWAY_TOKEN` outside interactive use.
    pub gateway_token: Option<String>,
    /// Explicit gateway.json path.
    pub discovery_path: Option<String>,
    /// Permit plaintext HTTP to a remote host.
    #[serde(default)]
    pub allow_insecure_http: bool,
    /// Stable key required by every mutation.
    pub idempotency_key: Option<String>,
    /// Explicitly admit destructive or unknown raw commands.
    #[serde(default)]
    pub unsafe_mode: bool,
}

#[derive(Clone, Debug, Deserialize, incurs::Args)]
pub struct RawCallArgs {
    /// Exact host gateway command name.
    pub command: String,
}

#[derive(Clone, Debug, JsonSchema, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallOutput {
    pub command: String,
    pub effect: CommandEffect,
    pub open_world: bool,
    pub replayed: bool,
    pub result: Value,
    pub receipt: Option<MutationReceipt>,
}

#[derive(Debug, Error)]
pub enum CallError {
    #[error(transparent)]
    Policy(#[from] PolicyError),
    #[error("request body is not valid JSON: {0}")]
    Body(#[from] serde_json::Error),
    #[error("gateway configuration failed: {0}")]
    Discovery(#[from] grokctl_core::config::DiscoveryError),
    #[error("gateway call failed: {0}")]
    Gateway(#[from] grokctl_core::gateway::GatewayError),
    #[error("receipt journal failed: {0}")]
    Journal(#[from] grokctl_core::receipt::JournalError),
    #[error("could not prepare receipt directory: {0}")]
    ReceiptDirectory(std::io::Error),
}

struct Mutation<'a> {
    command: &'a str,
    policy: &'a CommandPolicy,
    gateway_id: &'a str,
    key: &'a str,
    client: &'a GatewayClient,
    journal: ReceiptJournal,
}

pub type FixedSpec = (&'static str, &'static str, &'static str);

pub fn fixed_group(name: &'static str, description: &'static str, specs: &[FixedSpec]) -> Cli {
    specs.iter().fold(
        Cli::create(name).description(description),
        |cli, (cli_name, host_name, command_description)| {
            cli.command(*cli_name, fixed_command(cli_name, host_name, command_description))
        },
    )
}

pub fn fixed_command(
    cli_name: &'static str,
    host_name: &'static str,
    description: &'static str,
) -> CommandDef {
    let policy = classify_command(host_name);
    CommandDef::typed::<(), CallOptions, (), CallOutput, _, _>(cli_name, move |ctx| async move {
        typed_result(call_host(host_name, &ctx.options).await)
    })
    .description(description)
    .mcp(mcp_options(&policy, description))
    .destructive(policy.effect == CommandEffect::Destructive)
    .done()
}

pub async fn raw_call(
    context: TypedContext<RawCallArgs, CallOptions, ()>,
) -> TypedResult<CallOutput> {
    typed_result(call_host(&context.args.command, &context.options).await)
}

pub async fn call_host_json(
    command: &str,
    options: &CallOptions,
    body: Value,
) -> TypedResult<CallOutput> {
    let mut options = options.clone();
    options.body = Some(body.to_string());
    typed_result(call_host(command, &options).await)
}

pub async fn call_host_empty(command: &str, options: &CallOptions) -> TypedResult<CallOutput> {
    typed_result(call_host(command, options).await)
}

pub async fn call_host(command: &str, options: &CallOptions) -> Result<CallOutput, CallError> {
    let policy = classify_command(command);
    enforce_policy(&policy, options)?;
    let body = options.body.as_deref().map(serde_json::from_str).transpose()?;
    let resolved = discover_gateway(&discover_options(options))?;
    let gateway_id = resolved.base_url.as_str().to_owned();
    let client = GatewayClient::new(resolved, GatewayClientOptions::default())?;
    if policy.effect == CommandEffect::Read {
        let result = client.command(command, body.as_ref()).await?;
        return Ok(output(command, &policy, false, result, None));
    }
    let mutation = Mutation {
        command,
        policy: &policy,
        gateway_id: &gateway_id,
        key: options.idempotency_key.as_deref().unwrap_or_default(),
        client: &client,
        journal: open_journal()?,
    };
    mutation.run(body).await
}

fn enforce_policy(policy: &CommandPolicy, options: &CallOptions) -> Result<(), PolicyError> {
    if policy.effect == CommandEffect::Excluded {
        return Err(PolicyError::Excluded(policy.name.clone()));
    }
    if policy.effect == CommandEffect::Destructive && !options.unsafe_mode {
        return Err(PolicyError::UnsafeRequired(policy.name.clone()));
    }
    if policy.effect != CommandEffect::Read && options.idempotency_key.as_deref().is_none() {
        return Err(PolicyError::IdempotencyRequired(policy.name.clone()));
    }
    Ok(())
}

fn discover_options(options: &CallOptions) -> DiscoverOptions {
    DiscoverOptions {
        url: options.gateway_url.clone(),
        token: options.gateway_token.clone(),
        discovery_path: options.discovery_path.as_ref().map(PathBuf::from),
        env: std::env::vars().collect::<HashMap<_, _>>(),
        allow_insecure_http: options.allow_insecure_http,
    }
}

pub fn gateway_client(options: &CallOptions) -> Result<GatewayClient, CallError> {
    let resolved = discover_gateway(&discover_options(options))?;
    Ok(GatewayClient::new(resolved, GatewayClientOptions::default())?)
}

impl Mutation<'_> {
    async fn run(&self, body: Option<Value>) -> Result<CallOutput, CallError> {
        let witness = mutation_receipt(self.gateway_id, self.command, self.key, body.as_ref());
        let admitted = self.journal.begin(&witness)?;
        if admitted.state != ReceiptState::Started {
            return Ok(output(self.command, self.policy, true, Value::Null, Some(admitted)));
        }
        let result = self.client.command(self.command, body.as_ref()).await;
        self.finish(result)
    }

    fn finish(
        &self,
        result: Result<Value, grokctl_core::gateway::GatewayError>,
    ) -> Result<CallOutput, CallError> {
        match result {
            Ok(value) => self.succeed(value),
            Err(error) => {
                self.journal.mark_ambiguous(self.gateway_id, self.key)?;
                Err(CallError::Gateway(error))
            }
        }
    }

    fn succeed(&self, value: Value) -> Result<CallOutput, CallError> {
        let response_hash = canonical_json_hash(&value);
        self.journal.mark_succeeded(self.gateway_id, self.key, &response_hash)?;
        let receipt = self.journal.get(self.gateway_id, self.key)?;
        Ok(output(self.command, self.policy, false, value, receipt))
    }
}

fn mutation_receipt(
    gateway_id: &str,
    command: &str,
    key: &str,
    body: Option<&Value>,
) -> MutationReceipt {
    MutationReceipt {
        gateway_id: gateway_id.to_owned(),
        command: command.to_owned(),
        idempotency_key: key.to_owned(),
        input_hash: canonical_json_hash(body.unwrap_or(&Value::Null)),
        state: ReceiptState::Started,
        request_id: None,
        response_hash: None,
    }
}

pub fn open_journal() -> Result<ReceiptJournal, CallError> {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    let directory = base.join("grokctl");
    fs::create_dir_all(&directory).map_err(CallError::ReceiptDirectory)?;
    Ok(ReceiptJournal::open(directory.join("receipts.sqlite3"))?)
}

fn output(
    command: &str,
    policy: &CommandPolicy,
    replayed: bool,
    result: Value,
    receipt: Option<MutationReceipt>,
) -> CallOutput {
    CallOutput {
        command: command.to_owned(),
        effect: policy.effect,
        open_world: policy.open_world,
        replayed,
        result,
        receipt,
    }
}

fn typed_result(result: Result<CallOutput, CallError>) -> TypedResult<CallOutput> {
    match result {
        Ok(value) => TypedResult::ok(value),
        Err(error) => TypedResult::error("GROKCTL_ERROR", error.to_string()),
    }
}

pub fn mcp_options(policy: &CommandPolicy, description: &str) -> McpCommandOptions {
    McpCommandOptions {
        description: Some(description.to_owned()),
        annotations: Some(McpAnnotations {
            title: None,
            read_only_hint: Some(policy.effect == CommandEffect::Read),
            destructive_hint: Some(policy.effect == CommandEffect::Destructive),
            idempotent_hint: Some(policy.effect != CommandEffect::Read),
            open_world_hint: Some(policy.open_world),
        }),
        destructive: policy.effect == CommandEffect::Destructive,
        ..McpCommandOptions::default()
    }
}
