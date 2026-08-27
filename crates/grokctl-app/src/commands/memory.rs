//! Bot memory commands.

use incurs::cli::Cli;

use super::common::{FixedSpec, fixed_group};

const COMMANDS: &[FixedSpec] = &[
    ("list", "getAgentMemories", "List one Bot's memories"),
    ("delete", "deleteAgentMemory", "Delete one Bot memory"),
    ("clear", "clearAgentMemories", "Clear one Bot's memories"),
];

pub fn group() -> Cli {
    fixed_group("memory", "Manage Bot memories", COMMANDS)
}
