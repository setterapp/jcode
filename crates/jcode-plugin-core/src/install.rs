use anyhow::Result;
use std::path::PathBuf;

use crate::manifest::{PluginEntry, PluginManifest, PluginSource};

pub struct PluginInstaller;

impl PluginInstaller {
    pub fn new() -> Self {
        Self
    }

    pub async fn install(&self, spec: &str) -> Result<PluginEntry> {
        if spec.starts_with("file://") || spec.starts_with('.') || spec.starts_with('/') {
            self.install_file(spec).await
        } else if spec.starts_with("git+") || spec.starts_with("https://github.com") {
            self.install_git(spec).await
        } else {
            self.install_npm(spec).await
        }
    }

    pub async fn resolve(&self, spec: &str) -> Result<PluginEntry> {
        self.install(spec).await
    }

    async fn install_npm(&self, package: &str) -> Result<PluginEntry> {
        tracing::info!("Installing npm plugin: {}", package);

        let output = tokio::process::Command::new("npm")
            .args(["pack", "--dry-run", package])
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("npm not available: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("npm pack failed: {}", stderr.trim());
        }

        let manifest = PluginManifest {
            name: package.to_string(),
            version: "0.1.0".to_string(),
            description: Some(format!("npm plugin: {}", package)),
            server: true,
            tui: false,
            engines: None,
        };

        Ok(PluginEntry {
            manifest,
            source: PluginSource::Npm { package: package.to_string() },
            server_entry: None,
            tui_entry: None,
        })
    }

    async fn install_git(&self, spec: &str) -> Result<PluginEntry> {
        tracing::info!("Installing git plugin: {}", spec);
        let manifest = PluginManifest {
            name: spec.split('/').last().unwrap_or(spec).to_string(),
            version: "0.1.0".to_string(),
            description: Some(format!("git plugin: {}", spec)),
            server: true,
            tui: false,
            engines: None,
        };

        Ok(PluginEntry {
            manifest,
            source: PluginSource::Git {
                url: spec.to_string(),
                ref_name: None,
            },
            server_entry: None,
            tui_entry: None,
        })
    }

    async fn install_file(&self, path: &str) -> Result<PluginEntry> {
        let resolved = PathBuf::from(path);
        let manifest = PluginManifest {
            name: resolved
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            version: "0.1.0".to_string(),
            description: Some(format!("file plugin: {}", path)),
            server: true,
            tui: false,
            engines: None,
        };

        Ok(PluginEntry {
            manifest,
            source: PluginSource::File { path: path.to_string() },
            server_entry: None,
            tui_entry: None,
        })
    }
}
