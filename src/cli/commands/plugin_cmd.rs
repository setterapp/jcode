use anyhow::Result;
use crate::plugin::PluginManager;

pub async fn run_plugin_install(spec: &str, force: bool) -> Result<()> {
    let manager = PluginManager::new()?;

    if !force {
        let existing = manager.list()?;
        if existing.iter().any(|p| p == spec || p.starts_with(&format!("{}@", spec))) {
            anyhow::bail!("Plugin '{}' already installed. Use --force to reinstall.", spec);
        }
    }

    manager.install(spec).await?;

    let mut plugins = manager.list()?;
    if !plugins.iter().any(|p| p == spec) {
        plugins.push(spec.to_string());
        manager.save_plugin_list(&plugins)?;
    }

    Ok(())
}

pub fn run_plugin_list() -> Result<()> {
    let manager = PluginManager::new()?;
    let plugins = manager.list()?;

    if plugins.is_empty() {
        println!("No plugins installed.");
        println!("  Use: {}", crate::product::command_with("plug <npm-package>"));
        return Ok(());
    }

    println!("Installed plugins:");
    for p in &plugins {
        println!("  {}", p);
    }

    Ok(())
}
