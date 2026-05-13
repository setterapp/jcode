use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    #[serde(default)]
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpTransport {
    #[serde(rename = "stdio")]
    Stdio { command: String, args: Vec<String> },
    #[serde(rename = "sse")]
    Sse { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceDefinition {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptDefinition {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Vec<McpPromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpOAuthState {
    pub server_name: String,
    pub authorization_url: String,
    pub state: String,
    pub code_verifier: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerStatus {
    pub name: String,
    pub transport: String,
    pub connected: bool,
    pub tools: usize,
    pub resources: usize,
    pub prompts: usize,
    pub error: Option<String>,
}
