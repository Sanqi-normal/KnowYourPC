use std::sync::Arc;

use clap::Parser;

use fastscan_mcp::state::AppState;
use fastscan_mcp::tools::ToolRegistry;

mod server_http;
mod server_stdio;

#[derive(Parser)]
#[command(name = "fastscan-mcp", about = "FastScan MCP Server for disk analysis")]
struct Cli {
    #[arg(long, help = "Run in HTTP mode instead of stdio")]
    http: bool,

    #[arg(long, default_value = "3721", help = "HTTP server port")]
    port: u16,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let state = Arc::new(AppState::new());
    let tools = ToolRegistry::new(state.clone());

    if cli.http {
        server_http::serve(tools, cli.port).await;
    } else {
        server_stdio::serve(tools).await;
    }
}
