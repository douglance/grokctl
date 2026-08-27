//! Workflow commands.

use incurs::cli::Cli;

use super::common::{FixedSpec, fixed_group};

const COMMANDS: &[FixedSpec] = &[
    ("list", "getAgentWorkflows", "List one Bot's workflows"),
    ("create", "createAgentWorkflow", "Create a workflow"),
    ("update", "updateAgentWorkflow", "Update a workflow"),
    ("delete", "deleteAgentWorkflow", "Delete a workflow"),
    ("run", "runAgentWorkflowNow", "Run a workflow now"),
    ("enable", "setAgentWorkflowEnabled", "Enable or disable a workflow"),
    ("import-text", "importAgentWorkflowText", "Import workflow text"),
    ("import-url", "importAgentWorkflowUrl", "Import a workflow URL"),
    ("port-skills", "portAgentLocalSkills", "Port local skills into workflows"),
];

pub fn group() -> Cli {
    fixed_group("workflow", "Manage Bot workflows", COMMANDS)
}
