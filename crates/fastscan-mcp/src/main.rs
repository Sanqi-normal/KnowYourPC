use std::sync::Arc;

use clap::Parser;

use fastscan_mcp::state::AppState;
use fastscan_mcp::tools::ToolRegistry;
use fastscan_mcp::win::elevate::{elevate_self, is_elevated};

mod server_http;

#[derive(Parser)]
#[command(name = "fastscan-mcp", about = "FastScan MCP Server for disk analysis")]
struct Cli {
    #[arg(long, default_value = "3721", help = "HTTP server port")]
    port: u16,

    #[arg(long, help = "Skip auto-elevation at startup (for embedders)")]
    no_elevate: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if !cli.no_elevate && !is_elevated() {
        match elevate_self(cli.port) {
            Ok(()) => {
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("提权失败: {e}");
                eprintln!("以非管理员身份继续运行，NTFS MFT 扫描将不可用");
            }
        }
    }

    let state = Arc::new(AppState::new());
    let tools = ToolRegistry::new(state.clone());

    server_http::serve(tools, cli.port).await;
}
