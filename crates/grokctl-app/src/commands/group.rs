//! Bot group commands.

use incurs::cli::Cli;

use super::common::{FixedSpec, fixed_group};

const COMMANDS: &[FixedSpec] = &[
    ("list", "listAgents", "List roster rows, including groups"),
    ("create", "createGroup", "Create a Bot group"),
    ("set-members", "setGroupMembers", "Replace a Bot group's members"),
    ("broadcast", "broadcastToAgents", "Broadcast a prompt to Bots"),
];

pub fn group() -> Cli {
    fixed_group("group", "Manage Bot groups", COMMANDS)
}
