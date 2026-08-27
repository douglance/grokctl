//! Grok Bot computer commands.

use incurs::cli::Cli;

use super::common::{FixedSpec, fixed_group};

const COMMANDS: &[FixedSpec] = &[
    ("box-status", "getForeverBoxStatus", "Read forever-box status"),
    ("store-status", "getBoxStoreStatus", "Read box-store status"),
    ("subagents", "getSubagents", "List a Bot's subagents"),
    ("tasks", "getAsyncTasks", "List a Bot's asynchronous tasks"),
    ("respond", "respondToWidget", "Respond to a Bot widget"),
];

pub fn group() -> Cli {
    fixed_group("computer", "Inspect the shared Bot computer", COMMANDS)
}
