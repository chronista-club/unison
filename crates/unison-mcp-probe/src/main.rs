//! unison-mcp-probe
//!
//! Unison Protocol サーバを Claude Code から対話的につつくための MCP server。
//! stdio transport で起動し、`unison_ping` / `unison_call` / `unison_channel_list`
//! の 3 つの tool を提供する。
//!
//! # 使い方
//!
//! `.mcp.json` に以下を追加:
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "unison-probe": {
//!       "type": "stdio",
//!       "command": "cargo",
//!       "args": ["run", "-p", "unison-mcp-probe", "--release"]
//!     }
//!   }
//! }
//! ```

use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

mod tools;

#[tokio::main]
async fn main() -> Result<()> {
    // stdout は MCP transport に使うので、log は stderr に出す
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("unison-mcp-probe starting on stdio");

    let probe = tools::UnisonProbe::new();
    let server = probe
        .serve(stdio())
        .await
        .inspect_err(|e| tracing::error!("MCP serve error: {:?}", e))?;

    server.waiting().await?;
    Ok(())
}
