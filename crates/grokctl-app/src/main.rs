//! Grok Bot control CLI and MCP server entry point.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    grokctl::build_cli().serve().await
}
