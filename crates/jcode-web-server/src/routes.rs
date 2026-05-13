use axum::{
    Router,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    routing::{get, post},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::types::*;

/// Shared application state passed to all route handlers.
pub struct AppState {
    pub session_dir: PathBuf,
    pub models: Vec<ModelInfo>,
    pub daemon_socket_path: Option<PathBuf>,
}

impl AppState {
    /// Build an AppState from ServerOptions, resolving the session directory
    /// from the options, environment, or the default home-dir path.
    pub fn from_options(options: &ServerOptions) -> Self {
        let session_dir = options.session_dir.clone().unwrap_or_else(default_sessions_dir);
        let _ = std::fs::create_dir_all(&session_dir);

        Self {
            session_dir,
            models: default_models(),
            daemon_socket_path: options.daemon_socket_path.clone(),
        }
    }
}

/// Resolve the default sessions directory using JCODE_HOME, then ~/.jcode/sessions.
fn default_sessions_dir() -> PathBuf {
    if let Ok(path) = std::env::var("JCODE_HOME") {
        return PathBuf::from(path).join("sessions");
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".jcode").join("sessions");
    }
    // Last-resort fallback (unlikely on any real system)
    PathBuf::from("~/.jcode/sessions")
}

fn default_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { name: "claude-sonnet-4".into(), provider: "anthropic".into() },
        ModelInfo { name: "claude-3.5-haiku".into(), provider: "anthropic".into() },
        ModelInfo { name: "gpt-5.2-codex".into(), provider: "openai".into() },
        ModelInfo { name: "gpt-4o".into(), provider: "openai".into() },
        ModelInfo { name: "deepseek-chat".into(), provider: "deepseek".into() },
        ModelInfo { name: "gemini-2.5-pro".into(), provider: "google".into() },
    ]
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index_html_handler))
        .route("/health", get(health_handler))
        .route("/ws", get(ws_handler))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{id}", get(get_session_detail))
        .route("/api/sessions/{id}/message", post(send_message_to_session))
        .route("/api/models", get(list_models))
        .route("/api/config", get(get_config))
        .route("/api/stats", get(get_stats))
        .route("/api/version", get(version_handler))
        .with_state(state)
}

/// GET / — serve the embedded web UI HTML
async fn index_html_handler() -> (StatusCode, [(axum::http::HeaderName, &'static str); 2], &'static str) {
    (
        StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (axum::http::header::CACHE_CONTROL, "no-cache"),
        ],
        include_str!("../static/index.html"),
    )
}

/// GET /ws — WebSocket endpoint proxying to the daemon Unix socket.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut ws: WebSocket, state: Arc<AppState>) {
    let socket_path = match &state.daemon_socket_path {
        Some(p) => p.clone(),
        None => {
            let _ = ws.send(Message::Text(json!({
                "type": "error",
                "message": "No daemon configured (daemon_socket_path not set)"
            }).to_string().into())).await;
            return;
        }
    };

    // Connect to the daemon Unix socket
    let stream = match UnixStream::connect(&socket_path).await {
        Ok(s) => s,
        Err(e) => {
            let _ = ws.send(Message::Text(json!({
                "type": "error",
                "message": format!("Failed to connect to daemon: {}", e)
            }).to_string().into())).await;
            return;
        }
    };

    let (daemon_reader, mut daemon_writer) = stream.into_split();
    let mut daemon_reader = BufReader::new(daemon_reader);

    // Split the WebSocket for bidirectional forwarding
    let (ws_sender, mut ws_receiver) = ws.split();

    // Task 1: Forward WebSocket messages → daemon
    let forward_to_daemon = async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let line = format!("{}\n", text);
                    if daemon_writer.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                    if daemon_writer.flush().await.is_err() {
                        break;
                    }
                }
                Ok(Message::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
    };

    // Task 2: Forward daemon events → WebSocket
    let forward_to_ws = async move {
        let mut line_buf = String::new();
        let mut ws_sender = ws_sender;
        loop {
            line_buf.clear();
            match daemon_reader.read_line(&mut line_buf).await {
                Ok(0) => break, // EOF
                Ok(_) => {
                    let trimmed = line_buf.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if ws_sender.send(Message::Text(trimmed.to_string().into())).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    };

    tokio::select! {
        _ = forward_to_daemon => {},
        _ = forward_to_ws => {},
    }
}

// ---------------------------------------------------------------------------
//  REST API Handlers
// ---------------------------------------------------------------------------

async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

/// GET /api/sessions — list all sessions by reading JSON files from the
/// sessions directory.
async fn list_sessions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SessionSummary>>, (StatusCode, Json<ApiError>)> {
    let mut summaries: Vec<SessionSummary> = Vec::new();

    let entries = match std::fs::read_dir(&state.session_dir) {
        Ok(e) => e,
        Err(e) => {
            return Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to read sessions directory: {}", e),
            ));
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        // Skip backup files
        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
            if name.ends_with(".bak") || name.ends_with(".tmp") {
                continue;
            }
        }

        match std::fs::read_to_string(&path) {
            Ok(data) => {
                match serde_json::from_str::<Value>(&data) {
                    Ok(obj) => {
                        let id = obj
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let title = obj
                            .get("title")
                            .and_then(|v| v.as_str())
                            .or_else(|| obj.get("custom_title").and_then(|v| v.as_str()))
                            .map(|s| s.to_string());
                        let created_at = obj
                            .get("created_at")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let messages = obj
                            .get("messages")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);

                        summaries.push(SessionSummary {
                            id,
                            title,
                            created_at,
                            message_count: messages,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Skipping corrupt session file {}: {}",
                            path.display(),
                            e
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Skipping unreadable session file {}: {}",
                    path.display(),
                    e
                );
            }
        }
    }

    // Sort by creation date descending (newest first)
    summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(Json(summaries))
}

/// GET /api/sessions/{id} — get full session detail with messages.
async fn get_session_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionDetail>, (StatusCode, Json<ApiError>)> {
    // Sanitize the id: only allow alphanumeric, hyphens, underscores, dots
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Invalid session ID format",
        ));
    }

    let session_path = state.session_dir.join(format!("{}.json", id));

    let data = match std::fs::read_to_string(&session_path) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(api_error(
                StatusCode::NOT_FOUND,
                &format!("Session '{}' not found", id),
            ));
        }
        Err(e) => {
            return Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to read session: {}", e),
            ));
        }
    };

    let obj: Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(e) => {
            return Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to parse session data: {}", e),
            ));
        }
    };

    let messages = obj
        .get("messages")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let message_count = messages.len();

    let detail = SessionDetail {
        id: obj.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        title: obj
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        custom_title: obj
            .get("custom_title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        created_at: obj
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        updated_at: obj
            .get("updated_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        messages,
        model: obj
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        provider_key: obj
            .get("provider_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        status: obj
            .get("status")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .or_else(|| {
                obj.get("status")
                    .and_then(|v| v.get("kind").and_then(|k| k.as_str()))
                    .map(|s| s.to_string())
            }),
        message_count,
    };

    Ok(Json(detail))
}

/// POST /api/sessions/{id}/message — send a message to a session.
async fn send_message_to_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<MessagePayload>,
) -> Result<Json<MessageSubmissionResponse>, (StatusCode, Json<ApiError>)> {
    // Sanitize the id
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "Invalid session ID format",
        ));
    }

    let session_path = state.session_dir.join(format!("{}.json", id));

    if !session_path.exists() {
        return Err(api_error(
            StatusCode::NOT_FOUND,
            &format!("Session '{}' not found", id),
        ));
    }

    let msg_id = uuid::Uuid::new_v4().to_string();

    tracing::info!(
        "Message submitted to session {}: {} chars",
        id,
        payload.content.len()
    );

    Ok(Json(MessageSubmissionResponse {
        id: msg_id,
        status: "received".to_string(),
    }))
}

/// GET /api/models — list available models.
async fn list_models(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<ModelInfo>> {
    Json(state.models.clone())
}

/// GET /api/config — get full configuration.
async fn get_config(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "name": "jcode-plus",
        "version": env!("CARGO_PKG_VERSION"),
        "session_dir": state.session_dir.to_string_lossy(),
        "model_count": state.models.len(),
        "daemon_connected": state.daemon_socket_path.is_some(),
    }))
}

/// GET /api/stats — get usage statistics computed from session files.
async fn get_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<ApiError>)> {
    let mut total_sessions = 0usize;
    let mut total_messages = 0usize;
    let mut active_sessions = 0usize;

    let entries = match std::fs::read_dir(&state.session_dir) {
        Ok(e) => e,
        Err(e) => {
            return Err(api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Failed to read sessions directory: {}", e),
            ));
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
            if name.ends_with(".bak") || name.ends_with(".tmp") {
                continue;
            }
        }

        match std::fs::read_to_string(&path) {
            Ok(data) => {
                if let Ok(obj) = serde_json::from_str::<Value>(&data) {
                    total_sessions += 1;
                    if let Some(msgs) = obj.get("messages").and_then(|v| v.as_array()) {
                        total_messages += msgs.len();
                    }
                    let is_active = match obj.get("status") {
                        Some(Value::String(s)) => s == "Active",
                        Some(Value::Object(o)) => o.get("kind").and_then(|k| k.as_str()) == Some("Active"),
                        _ => true,
                    };
                    if is_active {
                        active_sessions += 1;
                    }
                }
            }
            Err(_) => {}
        }
    }

    Ok(Json(StatsResponse {
        total_sessions,
        total_messages,
        active_sessions,
    }))
}

/// GET /api/version — version info.
async fn version_handler() -> Json<Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "build": "jcode-plus"
    }))
}

// ---------------------------------------------------------------------------
//  Helpers
// ---------------------------------------------------------------------------

fn api_error(status: StatusCode, message: &str) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error: message.to_string(),
            code: status.as_u16(),
        }),
    )
}
