use crate::types::McpToolDefinition;
use std::collections::HashMap;

#[derive(Default)]
pub struct McpRegistry {
    servers: HashMap<String, Vec<McpToolDefinition>>,
}

impl McpRegistry {
    pub fn new() -> Self {
        Self { servers: HashMap::new() }
    }

    pub fn register_tool(&mut self, tool: McpToolDefinition) {
        self.servers
            .entry("default".to_string())
            .or_default()
            .push(tool);
    }

    pub fn register_tools(&mut self, server: &str, tools: Vec<McpToolDefinition>) {
        self.servers.entry(server.to_string()).or_default().extend(tools);
    }

    pub fn tools(&self) -> &[McpToolDefinition] {
        self.servers
            .get("default")
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn all_tools(&self) -> Vec<(&str, &McpToolDefinition)> {
        self.servers
            .iter()
            .flat_map(|(server, tools)| {
                tools.iter().map(move |t| (server.as_str(), t))
            })
            .collect()
    }

    pub fn server_tools(&self, server: &str) -> Option<&[McpToolDefinition]> {
        self.servers.get(server).map(|v| v.as_slice())
    }

    pub fn servers(&self) -> impl Iterator<Item = &str> {
        self.servers.keys().map(|s| s.as_str())
    }

    pub fn clear(&mut self) {
        self.servers.clear();
    }
}
