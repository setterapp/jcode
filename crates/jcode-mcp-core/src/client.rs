use anyhow::Result;
use serde_json::{json, Value};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::transport::{StdioTransport, TransportEvent};
use crate::types::*;
use crate::registry::McpRegistry;

pub struct McpClient {
    name: String,
    transport: StdioTransport,
    event_rx: mpsc::UnboundedReceiver<TransportEvent>,
    pending: std::collections::HashMap<String, tokio::sync::oneshot::Sender<Result<Value>>>,
    registry: McpRegistry,
}

impl McpClient {
    pub async fn connect(config: &McpServerConfig) -> Result<Self> {
        match &config.transport {
            McpTransport::Stdio { command, args } => {
                let (transport, event_rx) = StdioTransport::connect(command, args).await?;
                let mut client = Self {
                    name: config.name.clone(),
                    transport,
                    event_rx,
                    pending: std::collections::HashMap::new(),
                    registry: McpRegistry::new(),
                };
                client.initialize().await?;
                client.list_tools().await?;
                Ok(client)
            }
            McpTransport::Sse { url: _ } => {
                Err(anyhow::anyhow!("SSE transport not yet implemented"))
            }
        }
    }

    async fn initialize(&mut self) -> Result<()> {
        let result = self.send_request("initialize", json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "jcode-plus",
                "version": env!("CARGO_PKG_VERSION")
            }
        })).await?;

        let _server_caps = result.get("capabilities");
        Ok(())
    }

    async fn list_tools(&mut self) -> Result<()> {
        let result = self.send_request("tools/list", json!({})).await?;
        if let Some(tools) = result.get("tools").and_then(|v| v.as_array()) {
            for tool in tools {
                let def: McpToolDefinition = serde_json::from_value(tool.clone())?;
                self.registry.register_tool(def);
            }
        }
        Ok(())
    }

    pub async fn call_tool(&mut self, name: &str, args: Value) -> Result<Value> {
        self.send_request("tools/call", json!({
            "name": name,
            "arguments": args
        })).await
    }

    pub fn tools(&self) -> &[McpToolDefinition] {
        self.registry.tools()
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn registry(&self) -> &McpRegistry {
        &self.registry
    }

    async fn send_request(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        let request = json!({
            "jsonrpc": "2.0",
            "id": &id,
            "method": method,
            "params": params
        });

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.insert(id.clone(), tx);
        self.transport.send(&request.to_string()).await?;

        tokio::time::timeout(std::time::Duration::from_secs(30), rx).await
            .map_err(|_| anyhow::anyhow!("MCP request timed out for {}", method))?
            .map_err(|_| anyhow::anyhow!("MCP response channel closed"))?
    }

    fn handle_message(&mut self, msg: &str) {
        if let Ok(parsed) = serde_json::from_str::<Value>(msg) {
            if let Some(id) = parsed.get("id").and_then(|v| v.as_str()) {
                if let Some(tx) = self.pending.remove(id) {
                    if let Some(error) = parsed.get("error") {
                        let _ = tx.send(Err(anyhow::anyhow!("MCP error: {}", error)));
                    } else if let Some(result) = parsed.get("result") {
                        let _ = tx.send(Ok(result.clone()));
                    }
                }
            }
        }
    }
}
