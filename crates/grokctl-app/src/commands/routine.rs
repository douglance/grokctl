//! Routine and automation commands.

use incurs::cli::Cli;

use super::common::{FixedSpec, fixed_group};

const COMMANDS: &[FixedSpec] = &[
    ("list", "getAgentAutomations", "List one Bot's routines"),
    ("list-all", "listAllAutomations", "List all routines"),
    ("create", "createAgentAutomation", "Create a routine"),
    ("update", "updateAgentAutomation", "Update a routine"),
    ("delete", "deleteAgentAutomation", "Delete a routine"),
    ("run", "runAgentAutomationNow", "Run a routine now"),
    ("enable", "setAgentAutomationEnabled", "Enable or disable a routine"),
];

pub fn group() -> Cli {
    fixed_group("routine", "Manage Bot routines", COMMANDS)
}
