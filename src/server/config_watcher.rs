use notify::{EventKind, RecursiveMode, Watcher};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;

pub(super) async fn watch_config_files(
    mcp_pool: Arc<OnceCell<Arc<crate::mcp::SharedMcpPool>>>,
) {
    let dir = match crate::storage::jcode_dir() {
        Ok(d) => d,
        Err(e) => {
            crate::logging::warn(&format!(
                "config watcher: cannot determine jcode dir, skipping: {}",
                e
            ));
            return;
        }
    };

    // The jcode directory may not exist at server start (fresh install — it
    // gets created by `jcode login` / first config write). Polling here costs
    // nothing once started and is the only way to recover without leaving
    // hot-reload permanently broken until the next restart.
    if !dir.exists() {
        crate::logging::info(&format!(
            "config watcher: dir {:?} does not exist yet, polling…",
            dir
        ));
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            if dir.exists() {
                crate::logging::info(&format!(
                    "config watcher: dir {:?} appeared, continuing setup",
                    dir
                ));
                break;
            }
        }
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<notify::Result<notify::Event>>();

    let tx2 = tx.clone();
    let mut watcher = match notify::recommended_watcher(move |ev| {
        let _ = tx2.send(ev);
    }) {
        Ok(w) => w,
        Err(e) => {
            crate::logging::warn(&format!(
                "config watcher: failed to create watcher: {}",
                e
            ));
            return;
        }
    };

    if let Err(e) = watcher.watch(&dir, RecursiveMode::Recursive) {
        crate::logging::warn(&format!(
            "config watcher: failed to watch {:?}: {}",
            dir, e
        ));
        return;
    }

    crate::logging::info(&format!("config watcher: watching {:?}", dir));

    loop {
        let first = match rx.recv().await {
            Some(ev) => ev,
            None => break,
        };

        let mut batch = vec![first];

        // Drain additional events for 500ms to debounce rapid writes
        let deadline = tokio::time::Instant::now() + Duration::from_millis(500);
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Some(ev)) => batch.push(ev),
                _ => break,
            }
        }

        let mut config_changed = false;
        let mut mcp_changed = false;
        let mut skills_changed = false;

        for result in batch.into_iter().flatten() {
            // Skip access/metadata-only events
            if matches!(result.kind, EventKind::Access(_)) {
                continue;
            }
            for path in &result.paths {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name == "config.toml" {
                    config_changed = true;
                } else if name == "mcp.json" || name.ends_with(".mcp.json") {
                    mcp_changed = true;
                } else if path
                    .to_str()
                    .map(|p| p.contains("/skills/"))
                    .unwrap_or(false)
                {
                    skills_changed = true;
                }
            }
        }

        if config_changed {
            crate::config::reload_config();
            crate::logging::info("config watcher: config.toml reloaded");
        }

        if mcp_changed {
            if let Some(pool) = mcp_pool.get() {
                let (count, errors) = pool.reload().await;
                if errors.is_empty() {
                    crate::logging::info(&format!(
                        "config watcher: MCP reloaded ({} servers)",
                        count
                    ));
                } else {
                    crate::logging::warn(&format!(
                        "config watcher: MCP reloaded ({} servers, {} errors: {:?})",
                        count,
                        errors.len(),
                        errors
                    ));
                }
            }
        }

        if skills_changed {
            match crate::skill::SkillRegistry::shared_registry().try_write() {
                Ok(mut registry) => {
                    let count = registry.reload_all().unwrap_or(0);
                    crate::logging::info(&format!(
                        "config watcher: skills reloaded ({} skills)",
                        count
                    ));
                }
                Err(_) => {
                    crate::logging::warn(
                        "config watcher: skill registry locked, skipping skills reload",
                    );
                }
            }
        }
    }

    crate::logging::info("config watcher: channel closed, exiting");
}
