use std::sync::Arc;

use axum::{
    Router,
    extract::State,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json,
};
use futures::stream::{self, Stream, StreamExt};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use fastscan_mcp::tools::ToolRegistry;

#[derive(Clone)]
struct AppState {
    tools: Arc<ToolRegistry>,
    sse_tx: broadcast::Sender<String>,
}

pub async fn serve(tools: ToolRegistry, port: u16) {
    let (sse_tx, _) = broadcast::channel::<String>(32);
    let state = AppState {
        tools: Arc::new(tools),
        sse_tx,
    };

    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/messages", post(messages_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("127.0.0.1:{}", port);
    println!("MCP HTTP server listening on http://{}/sse", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> &'static str {
    "ok"
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let session_id = Uuid::new_v4().to_string();
    let rx = state.sse_tx.subscribe();
    let stream = BroadcastStream::new(rx);

    let initial = vec![
        Ok(Event::default().event("endpoint").data(format!("/messages?session_id={session_id}"))),
        Ok(Event::default().event("session_id").data(session_id)),
    ];

    let events = stream::iter(initial).chain(stream.map(|msg| {
        match msg {
            Ok(data) => Ok(Event::default().data(data)),
            Err(_) => Ok(Event::default().event("error").data("channel closed")),
        }
    }));

    Sse::new(events).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

async fn messages_handler(
    State(state): State<AppState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let method = body.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let id = body.get("id");

    match method {
        "initialize" => {
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "fastscan-mcp",
                        "version": "0.1.0"
                    }
                }
            });
            Json(response)
        }
        "notifications/initialized" => {
            Json(json!({ "jsonrpc": "2.0" }))
        }
        "tools/list" => {
            let tools = state.tools.list_tools();
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tools }
            });
            Json(response)
        }
        "tools/call" => {
            let name = body.get("params").and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
            let arguments = body.get("params").and_then(|p| p.get("arguments")).cloned().unwrap_or(json!({}));

            match state.tools.call_tool(name, arguments).await {
                Ok(content) => {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [
                                { "type": "text", "text": serde_json::to_string_pretty(&content).unwrap_or_default() }
                            ]
                        }
                    });
                    Json(response)
                }
                Err(error) => {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32603, "message": error }
                    });
                    Json(response)
                }
            }
        }
        _ => {
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {method}") }
            });
            Json(response)
        }
    }
}
