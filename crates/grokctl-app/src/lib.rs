//! Incurs command graph for the `grokctl` CLI and MCP server.

mod commands;

use incurs::cli::Cli;

/// Build the single command graph shared by CLI and MCP transports.
#[must_use]
pub fn build_cli() -> Cli {
    commands::groups().into_iter().fold(
        Cli::create("grokctl")
            .version(env!("CARGO_PKG_VERSION"))
            .description("Control an authorized Grok Bot Sand gateway"),
        Cli::group,
    )
}
