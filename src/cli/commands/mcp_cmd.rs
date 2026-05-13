use anyhow::Result;
use crate::mcp::protocol::{McpConfig, McpServerConfig};
use crate::storage;

pub async fn run_mcp_list() -> Result<()> {
    let config = McpConfig::load();

    if config.servers.is_empty() {
        println!("No MCP servers configured.");
        println!(
            "  Use: {}",
            crate::product::command_with("mcp add <name> --command <cmd> [--args ...]")
        );
        return Ok(());
    }

    for (name, cfg) in &config.servers {
        println!("  {}  (stdio: {} {})", name, cfg.command, cfg.args.join(" "));
    }

    println!("\nTotal: {} server(s)", config.servers.len());
    Ok(())
}

pub async fn run_mcp_add(
    name: &str,
    command: Option<String>,
    args: &[String],
    _url: Option<String>,
) -> Result<()> {
    let mut config = McpConfig::load();

    let cmd = command.ok_or_else(|| anyhow::anyhow!("--command is required for stdio transport"))?;

    config.servers.insert(name.to_string(), McpServerConfig {
        command: cmd,
        args: args.to_vec(),
        env: Default::default(),
        shared: true,
    });

    let jcode_dir = storage::jcode_dir()?;
    let mcp_path = jcode_dir.join("mcp.json");
    config.save_to_file(&mcp_path)?;
    println!("Added MCP server: {}", name);
    Ok(())
}

pub async fn run_mcp_debug(name: &str) -> Result<()> {
    let config = McpConfig::load();

    let Some(server_config) = config.servers.get(name) else {
        anyhow::bail!("MCP server '{}' not found in config", name);
    };

    println!("Connecting to MCP server: {}...", name);
    let _manager = crate::mcp::McpManager::new();
    _manager.connect(name, server_config).await?;
    println!("Connected successfully!");
    Ok(())
}

pub async fn run_mcp_auth(name: Option<String>) -> Result<()> {
    let config = McpConfig::load();

    match name {
        Some(n) => {
            if config.servers.contains_key(&n) {
                println!("Server '{}' exists.", n);
                println!("OAuth: not available for stdio transport");
            } else {
                println!("Server '{}' not found.", n);
            }
        }
        None => {
            if config.servers.is_empty() {
                println!("No MCP servers configured.");
            } else {
                println!("MCP servers:");
                for (name, _) in &config.servers {
                    println!("  {}", name);
                }
            }
        }
    }
    Ok(())
}

pub async fn run_mcp_logout(name: &str) -> Result<()> {
    println!("Removing OAuth credentials for MCP server: {}", name);
    println!("(OAuth logout not implemented for stdio transport)");
    Ok(())
}
