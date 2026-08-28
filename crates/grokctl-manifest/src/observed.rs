//! Commands observed in the authorized Grok Bot 0.29.0 build.
//!
//! The inspected `app.asar` SHA-256 is recorded in `docs/research.md`.

const BOT_TEMPLATE_COMMANDS: &[&str] = &[
    "createAgentFromTemplate",
    "publishBotTemplate",
    "listBotTemplates",
    "getBotTemplateVersion",
    "getBotTemplateForSourceAgent",
    "deleteBotTemplate",
    "setBotTemplateVisibility",
];

pub fn is_observed_command(name: &str) -> bool {
    BOT_TEMPLATE_COMMANDS.contains(&name)
}
