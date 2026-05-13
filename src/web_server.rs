use anyhow::Result;
use jcode_web_server::{ServerOptions, WebServer};

pub async fn start_web_server(port: u16, hostname: &str, open_browser: bool) -> Result<()> {
    // Resolve sessions directory using the main crate's storage
    let session_dir = crate::storage::jcode_dir()
        .ok()
        .map(|d| d.join("sessions"));

    // Resolve daemon socket path
    let daemon_socket_path = crate::server::socket_path();

    let options = ServerOptions {
        port,
        hostname: hostname.to_string(),
        cors_origins: vec![],
        open_browser,
        session_dir,
        daemon_socket_path: Some(daemon_socket_path),
    };

    let server = WebServer::new(options);

    if open_browser {
        let url = server.start_with_url().await?;
        println!("Web UI: {}", url);
    } else {
        println!("Server starting on {}:{}", hostname, port);
        server.start().await?;
    }

    Ok(())
}
