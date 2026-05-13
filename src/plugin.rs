use std::path::PathBuf;
use anyhow::Result;
use jcode_plugin_core::PluginInstaller;

pub struct PluginManager {
    installer: PluginInstaller,
    config_path: PathBuf,
}

impl PluginManager {
    pub fn new() -> Result<Self> {
        let flavor = crate::storage::product_flavor();
        let config_dir = dirs::config_dir()
            .map(|d| d.join(flavor.config_dir_name()))
            .unwrap_or_else(|| PathBuf::from(format!("~/.config/{}", flavor.config_dir_name())));
        std::fs::create_dir_all(&config_dir)?;

        Ok(Self {
            installer: PluginInstaller::new(),
            config_path: config_dir.join("plugins.json"),
        })
    }

    pub async fn install(&self, spec: &str) -> Result<()> {
        let entry = self.installer.install(spec).await?;
        println!("Installed plugin: {} v{}", entry.manifest.name, entry.manifest.version);
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<String>> {
        if !self.config_path.exists() {
            return Ok(vec![]);
        }
        let content = std::fs::read_to_string(&self.config_path)?;
        let plugins: Vec<String> = serde_json::from_str(&content)?;
        Ok(plugins)
    }

    pub fn save_plugin_list(&self, plugins: &[String]) -> Result<()> {
        let content = serde_json::to_string_pretty(plugins)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }
}
