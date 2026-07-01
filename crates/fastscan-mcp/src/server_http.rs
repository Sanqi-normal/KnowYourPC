use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{
    Router,
    extract::{Query, State},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json,
};
use futures::stream::{self, Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

use fastscan_mcp::tools::ToolRegistry;

#[derive(Clone)]
struct AppState {
    tools: Arc<ToolRegistry>,
    sessions: Arc<Mutex<HashMap<String, broadcast::Sender<String>>>>,
}

#[derive(Deserialize)]
struct MessageQuery {
    session_id: Option<String>,
}

pub async fn serve(tools: ToolRegistry, port: u16) {
    let state = AppState {
        tools: Arc::new(tools),
        sessions: Arc::new(Mutex::new(HashMap::new())),
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
    let (tx, rx) = broadcast::channel::<String>(32);
    state.sessions.lock().unwrap().insert(session_id.clone(), tx);
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
    Query(query): Query<MessageQuery>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let method = body.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let id = body.get("id");

    let response = match method {
        "initialize" => json!({
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
        }),
        "notifications/initialized" => json!({ "jsonrpc": "2.0" }),
        "tools/list" => {
            let tools = state.tools.list_tools();
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tools }
            })
        }
        "tools/call" => {
            let name = body.get("params").and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
            let arguments = body.get("params").and_then(|p| p.get("arguments")).cloned().unwrap_or(json!({}));

            match state.tools.call_tool(name, arguments).await {
                Ok(content) => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            { "type": "text", "text": serde_json::to_string_pretty(&content).unwrap_or_default() }
                        ]
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

    if id.is_some() {
        if let Some(session_id) = &query.session_id {
            let tx_opt = state.sessions.lock().unwrap().get(session_id).cloned();
            if let Some(tx) = tx_opt {
                if let Ok(response_str) = serde_json::to_string(&response) {
                    if tx.send(response_str).is_err() {
                        state.sessions.lock().unwrap().remove(session_id);
                    }
                }
            }
        }
    }

    Json(json!({ "jsonrpc": "2.0" }))
}
