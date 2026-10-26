//! `dd-mcp`: serves the map design tools over stdio.

use anyhow::Context as _;
use dd_mcp::server::DdMapperServer;
use dd_setup::Config;
use rmcp::ServiceExt as _;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_args(std::env::args().skip(1))?;
    let workspace = config.build_workspace()?;
    // Stdout carries the protocol: progress goes to stderr.
    eprintln!(
        "dd-mcp: {} assets in the catalog",
        workspace.catalog().len()
    );
    let service = DdMapperServer::new(workspace)
        .serve(rmcp::transport::stdio())
        .await
        .context("cannot start the MCP server on stdio")?;
    service.waiting().await?;
    Ok(())
}
