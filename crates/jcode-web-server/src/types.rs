use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerOptions {
    pub port: u16,
    pub hostname: String,
    pub cors_origins: Vec<String>,
    pub open_browser: bool,
    /// Optional override for the sessions directory.
    /// If None, resolved from JCODE_HOME env var or ~/.jcode/sessions at startup.
    pub session_dir: Option<PathBuf>,
    /// Path to the daemon Unix socket for real-time communication.
    /// If Some, the web server's WebSocket endpoint proxies to this socket.
    pub daemon_socket_path: Option<PathBuf>,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            port: 4096,
            hostname: "127.0.0.1".to_string(),
            cors_origins: vec![],
            open_browser: false,
            session_dir: None,
            daemon_socket_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: Option<String>,
    pub created_at: String,
    pub message_count: usize,
}

/// Full session detail including messages and metadata.
/// Deserialized from the stored session JSON files under the sessions directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDetail {
    pub id: String,
    pub title: Option<String>,
    pub custom_title: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages: Vec<serde_json::Value>,
    pub model: Option<String>,
    pub provider_key: Option<String>,
    pub status: Option<String>,
    pub message_count: usize,
}

/// Represents an available model for use in sessions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub provider: String,
}

/// Response returned after submitting a message to a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSubmissionResponse {
    pub id: String,
    pub status: String,
}

/// Payload for sending a message to a session via POST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub content: String,
    #[serde(default)]
    pub role: Option<String>,
}

/// API usage statistics computed from available session data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsResponse {
    pub total_sessions: usize,
    pub total_messages: usize,
    pub active_sessions: usize,
}

/// Generic API error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub code: u16,
}
