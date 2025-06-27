use use_aws_mcp::mcp_server::AwsMcpServer;
use use_aws_mcp::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("use_aws=info")
        .init();

    tracing::info!("Starting use_aws MCP server...");

    let mut server = AwsMcpServer::new();
    
    if let Err(e) = server.run().await {
        tracing::error!("Server error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}
