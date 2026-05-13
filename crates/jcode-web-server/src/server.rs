use anyhow::Result;
use axum::serve;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::info;

use crate::routes;
use crate::types::ServerOptions;

pub struct WebServer {
    options: ServerOptions,
}

impl WebServer {
    pub fn new(options: ServerOptions) -> Self {
        Self { options }
    }

    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.options.hostname, self.options.port);
        let listener = TcpListener::bind(&addr).await?;
        let actual_port = listener.local_addr()?.port();

        info!(
            "Web server listening on http://{}:{}",
            self.options.hostname, actual_port
        );

        let state = Arc::new(routes::AppState::from_options(&self.options));
        let app = routes::create_router(state);

        serve(listener, app).await?;

        Ok(())
    }

    pub async fn start_with_url(&self) -> Result<String> {
        let addr = format!("{}:{}", self.options.hostname, self.options.port);
        let listener = TcpListener::bind(&addr).await?;
        let actual_port = listener.local_addr()?.port();
        let url = format!("http://{}:{}", self.options.hostname, actual_port);

        info!("Web server listening on {}", url);

        if self.options.open_browser {
            if let Err(e) = open::that(&url) {
                tracing::warn!("Failed to open browser: {}", e);
            }
        }

        let state = Arc::new(routes::AppState::from_options(&self.options));
        let app = routes::create_router(state);

        serve(listener, app).await?;

        Ok(url)
    }
}
