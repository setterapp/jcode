use anyhow::Result;
use crate::web_server;
use super::super::provider_init::ProviderChoice;

pub async fn run_serve(port: u16, hostname: &str, open: bool) -> Result<()> {
    web_server::start_web_server(port, hostname, open).await
}

pub async fn run_web(port: u16, hostname: &str) -> Result<()> {
    println!("Starting jcode-plus Web UI...");

    // Spawn the daemon if not already running (same as `jcode` with no subcommand)
    // This ensures the WebSocket proxy has a backend to connect to
    let running = super::super::dispatch::server_is_running().await;
    if !running {
        println!("Starting jcode daemon...");
        super::super::dispatch::spawn_server(
            &ProviderChoice::Auto,
            None,
            None,
        ).await?;
    }

    web_server::start_web_server(port, hostname, true).await
}
