//! Markers recovered from the Grok Bot host bundle.

/// Start of the host gateway command table.
pub const COMMANDS_START: &str = "var SAND_GATEWAY_COMMANDS = {";

/// Start of the slim command table, which terminates the primary table.
pub const COMMANDS_END: &str = "var SAND_GATEWAY_SLIM_COMMANDS";

/// Start of the host capabilities declaration.
pub const CAPABILITIES_START: &str = "var HOST_CAPABILITIES = [";
