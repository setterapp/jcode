use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(default)]
    pub server: bool,
    #[serde(default)]
    pub tui: bool,
    #[serde(default)]
    pub engines: Option<PluginEngines>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEngines {
    pub opencode: Option<String>,
    pub jcode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub manifest: PluginManifest,
    pub source: PluginSource,
    pub server_entry: Option<String>,
    pub tui_entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginSource {
    Npm { package: String },
    Git { url: String, ref_name: Option<String> },
    File { path: String },
}
