//! Gateway command effect classification.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Effect class used by CLI validation and MCP annotations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandEffect {
    /// The command only observes state.
    Read,
    /// The command changes state without being inherently destructive.
    Mutation,
    /// The command can delete, reset, publish, or expose sensitive state.
    Destructive,
    /// Grokctl intentionally does not operate this host layer.
    Excluded,
}

/// Policy record attached to a gateway command.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPolicy {
    /// Exact host command name.
    pub name: String,
    /// Command effect class.
    pub effect: CommandEffect,
    /// Whether the command may interact with an external entity.
    pub open_world: bool,
}

/// Policy validation failure.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum PolicyError {
    /// An excluded command was requested.
    #[error("{0} belongs to Grok Bot's approval layer and is not available")]
    Excluded(String),
    /// A destructive or unknown command lacked explicit unsafe opt-in.
    #[error("{0} requires unsafe=true")]
    UnsafeRequired(String),
    /// A mutating command lacked an idempotency key.
    #[error("{0} requires an idempotency key")]
    IdempotencyRequired(String),
}

/// Classify a host command conservatively.
#[must_use]
pub fn classify_command(name: &str) -> CommandPolicy {
    let effect = match name {
        "resolveAutoReviewApproval" | "resolveLocalToolPermission" => CommandEffect::Excluded,
        _ if is_destructive_command(name) => CommandEffect::Destructive,
        _ if is_read_command(name) => CommandEffect::Read,
        _ if crate::seed::is_seed_command(name) => CommandEffect::Mutation,
        _ => CommandEffect::Destructive,
    };
    let open_world = is_open_world(name) || !crate::seed::is_seed_command(name);
    CommandPolicy { name: name.to_owned(), effect, open_world }
}

fn is_read_command(name: &str) -> bool {
    crate::seed::is_seed_command(name)
        && (name.starts_with("get")
            || name.starts_with("list")
            || name.starts_with("count")
            || name.starts_with("search")
            || name.starts_with("read")
            || name.starts_with("is")
            || matches!(name, "skillsCatalog" | "promptAcceptanceStatus"))
}

fn is_open_world(name: &str) -> bool {
    matches!(
        name,
        "sendPrompt"
            | "broadcastToAgents"
            | "runAgentAutomationNow"
            | "runAgentWorkflowNow"
            | "executeRoutedMcpTool"
            | "publishSkill"
            | "connectChannel"
    )
}

fn is_destructive_command(name: &str) -> bool {
    matches!(
        name,
        "deleteAgent"
            | "deleteAgents"
            | "deleteAgentMemory"
            | "clearAgentMemories"
            | "deleteAgentAutomation"
            | "deleteAgentWorkflow"
            | "resetForeverBox"
            | "clearBoxStoreNow"
            | "updateHostNow"
            | "autoUpdateBoxNow"
            | "handBackForeverBox"
            | "clearTrays"
            | "setBoxSecrets"
            | "submitSecret"
            | "completeMcpOAuth"
            | "requestWebAuthnCeremony"
            | "connectChannel"
            | "disconnectChannel"
            | "refreshChannel"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_commands_require_the_strongest_raw_call_guard() {
        let policy = classify_command("futureHostCommand");

        assert_eq!(policy.effect, CommandEffect::Destructive);
        assert!(policy.open_world);
        assert_eq!(classify_command("getFutureHostThing").effect, CommandEffect::Destructive);
        assert_eq!(classify_command("getHostStatus").effect, CommandEffect::Read);
        assert_eq!(classify_command("createAgent").effect, CommandEffect::Mutation);
        assert_eq!(classify_command("deleteAgent").effect, CommandEffect::Destructive);
    }
}
