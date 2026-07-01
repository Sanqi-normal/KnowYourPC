// Stdio transport implementation.
// Currently not compiled into the default binary (HTTP-only mode).
// Kept for reference — if you need subprocess-based MCP launching, you can
// wire this back by adding `mod server_stdio;` and re-adding the `--http` flag.
use std::io::{self, BufRead, Write};
use std::sync::Arc;

use serde_json::{json, Value};

use fastscan_mcp::tools::ToolRegistry;

pub async fn serve(tools: ToolRegistry) {
    let tools = Arc::new(tools);
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() { continue; }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = request.get("id");

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "fastscan-mcp", "version": "0.1.0" }
                }
            }),
            "notifications/initialized" => json!({ "jsonrpc": "2.0" }),
            "tools/list" => {
                let tools_list = tools.list_tools();
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "tools": tools_list }
                })
            }
            "tools/call" => {
                let name = request.get("params").and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
                let arguments = request.get("params").and_then(|p| p.get("arguments")).cloned().unwrap_or(json!({}));
                match tools.call_tool(name, arguments).await {
                    Ok(content) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{ "type": "text", "text": serde_json::to_string_pretty(&content).unwrap_or_default() }]
                        }
                    }),
                    Err(error) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32603, "message": error }
                    }),
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {method}") }
            }),
        };

        let response_str = serde_json::to_string(&response).unwrap();
        let mut out = stdout.lock();
        writeln!(out, "{response_str}").unwrap();
        out.flush().unwrap();
    }
}
