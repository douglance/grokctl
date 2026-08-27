//! Sand gateway transport.

mod avatar;
mod client;
mod error;
mod events;
mod sse;

pub use avatar::AvatarImage;
pub use client::{GatewayClient, GatewayClientOptions};
pub use error::GatewayError;
pub use events::EventStream;
pub use sse::{SseDecoder, SseError};
