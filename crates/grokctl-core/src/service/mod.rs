//! Higher-level Grok Bot workflows.

mod bots;
mod prompt;

pub use bots::{BotService, ResolveBotError};
pub use prompt::{PromptResult, PromptStatus, PromptWaitOptions};
