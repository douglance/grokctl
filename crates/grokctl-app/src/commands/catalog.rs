//! Skill, plugin, and routed MCP commands.

use incurs::cli::Cli;

use super::common::{FixedSpec, fixed_group};

const COMMANDS: &[FixedSpec] = &[
    ("skills", "skillsCatalog", "Read the skills catalog"),
    ("sync", "syncPluginSkills", "Synchronize plugin skills"),
    ("sync-status", "getPluginSyncStatus", "Read plugin sync status"),
    ("publish-targets", "getSkillPublishTargets", "List skill publish targets"),
    ("publish", "publishSkill", "Publish a skill"),
    ("resync", "resyncPublishedSkill", "Resynchronize a published skill"),
    ("unpublish", "unpublishSkill", "Unpublish a skill"),
    ("tools", "listRoutedMcpTools", "List routed MCP tools"),
    ("call-tool", "executeRoutedMcpTool", "Call a routed MCP tool"),
    ("box-servers", "listBoxMcpServers", "List box MCP servers"),
    ("refresh", "refreshMcp", "Refresh MCP state"),
];

pub fn group() -> Cli {
    fixed_group("catalog", "Inspect skills, plugins, and MCP tools", COMMANDS)
}
