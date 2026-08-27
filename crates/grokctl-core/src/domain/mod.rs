//! Typed Grok Bot gateway data.

mod bot;
mod event;
mod health;
mod host;

pub use bot::{BotSummary, PromptAccepted};
pub use event::GatewayEvent;
pub use health::HealthResponse;
pub use host::HostStatus;
