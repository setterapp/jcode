use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(default)]
    pub plugins: Vec<String>,
}

impl PluginConfig {
    pub fn has_plugin(&self, name: &str) -> bool {
        self.plugins.iter().any(|p| p == name || p.starts_with(&format!("{}@", name)))
    }

    pub fn add_plugin(&mut self, spec: &str) -> bool {
        if self.has_plugin(spec) {
            return false;
        }
        self.plugins.push(spec.to_string());
        true
    }

    pub fn remove_plugin(&mut self, name: &str) -> bool {
        let len = self.plugins.len();
        self.plugins.retain(|p| p != name && !p.starts_with(&format!("{}@", name)));
        self.plugins.len() < len
    }
}
