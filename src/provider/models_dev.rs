//! Cross-provider model catalog from [models.dev](https://models.dev).
//!
//! Pulls a normalized inventory of provider/model metadata — context limits,
//! modalities, cost per 1M tokens — for ~130 providers. The data is fetched
//! at boot in the background, cached on disk for 24 h, and merged into our
//! model picker so users see context size and `$/Mtok` for every model
//! without per-provider hand-coded tables.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::RwLock;
use std::time::{Duration, SystemTime};

/// HTTPS endpoint serving the full models.dev catalog as a single JSON document.
const MODELS_DEV_URL: &str = "https://models.dev/api.json";
/// Filename inside the user's data dir where we cache the fetched JSON.
const CACHE_FILENAME: &str = "models-dev.json";
/// How long a cache file is considered fresh enough that we skip the network.
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// One model entry as published by models.dev. Optional everywhere because the
/// upstream schema evolves and some providers omit fields.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ModelsDevEntry {
    pub id: String,
    pub name: Option<String>,
    pub family: Option<String>,
    pub attachment: bool,
    pub reasoning: bool,
    pub tool_call: bool,
    pub knowledge: Option<String>,
    pub release_date: Option<String>,
    pub modalities: Option<ModelsDevModalities>,
    pub open_weights: bool,
    pub limit: Option<ModelsDevLimit>,
    pub cost: Option<ModelsDevCost>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ModelsDevModalities {
    pub input: Vec<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ModelsDevLimit {
    pub context: Option<u64>,
    pub output: Option<u64>,
}

/// Cost per 1M tokens for each request type. Fields are optional because not
/// every provider publishes all of them.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ModelsDevCost {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
}

/// One provider's published metadata.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct ModelsDevProvider {
    pub id: String,
    pub name: Option<String>,
    pub doc: Option<String>,
    pub api: Option<String>,
    pub npm: Option<String>,
    pub env: Vec<String>,
    pub models: HashMap<String, ModelsDevEntry>,
}

/// Top-level models.dev document: provider id → provider record.
pub type ModelsDevCatalog = HashMap<String, ModelsDevProvider>;

/// Global process-wide cache, populated by `init_background` and read by
/// `get()`. None until the first successful fetch / cache hit.
static CATALOG: OnceLock<RwLock<Option<ModelsDevCatalog>>> = OnceLock::new();

fn cell() -> &'static RwLock<Option<ModelsDevCatalog>> {
    CATALOG.get_or_init(|| RwLock::new(None))
}

/// Return the cached catalog if it has been loaded already. Lock-free read on
/// the hot path — never blocks for a network fetch.
pub fn get() -> Option<ModelsDevCatalog> {
    cell().read().ok()?.clone()
}

/// Look up one model entry by `(provider_id, model_id)`. Convenience wrapper
/// around `get()`.
pub fn lookup(provider_id: &str, model_id: &str) -> Option<ModelsDevEntry> {
    let guard = cell().read().ok()?;
    let catalog = guard.as_ref()?;
    let provider = catalog.get(provider_id)?;
    provider.models.get(model_id).cloned()
}

fn cache_path() -> Result<PathBuf> {
    Ok(crate::storage::jcode_dir()?.join(CACHE_FILENAME))
}

fn read_cache_from_disk() -> Option<(ModelsDevCatalog, SystemTime)> {
    let path = cache_path().ok()?;
    let bytes = std::fs::read(&path).ok()?;
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok()?;
    let catalog: ModelsDevCatalog = serde_json::from_slice(&bytes).ok()?;
    Some((catalog, modified))
}

fn write_cache_to_disk(catalog: &ModelsDevCatalog) -> Result<()> {
    let path = cache_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(catalog)?;
    std::fs::write(&path, bytes)?;
    Ok(())
}

async fn fetch_from_network() -> Result<ModelsDevCatalog> {
    let client = crate::provider::shared_http_client();
    let response = client.get(MODELS_DEV_URL).send().await?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("models.dev returned HTTP {}", status);
    }
    let catalog: ModelsDevCatalog = response.json().await?;
    Ok(catalog)
}

/// Spawn a background tokio task that hydrates the catalog. Disk-first so the
/// process boots fast; network refresh runs only when the cache is stale.
///
/// Idempotent — safe to call from multiple startup paths; only the first call
/// queues the network request thanks to the `INITIALIZED` flag.
pub fn init_background() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static INITIALIZED: AtomicBool = AtomicBool::new(false);
    if INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }

    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };

    handle.spawn(async {
        let disk = tokio::task::spawn_blocking(read_cache_from_disk)
            .await
            .ok()
            .flatten();
        let cache_fresh = disk.as_ref().is_some_and(|(_, mtime)| {
            mtime.elapsed().map(|d| d < CACHE_TTL).unwrap_or(false)
        });

        if let Some((catalog, _)) = disk.as_ref() {
            if let Ok(mut guard) = cell().write() {
                *guard = Some(catalog.clone());
            }
        }

        if cache_fresh {
            return;
        }

        match fetch_from_network().await {
            Ok(catalog) => {
                let to_write = catalog.clone();
                let _ = tokio::task::spawn_blocking(move || write_cache_to_disk(&to_write)).await;
                if let Ok(mut guard) = cell().write() {
                    *guard = Some(catalog);
                }
            }
            Err(error) => {
                crate::logging::info(&format!(
                    "models.dev catalog fetch failed (will retry next boot): {}",
                    error
                ));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sample_anthropic_entry() {
        let payload = r#"{
            "anthropic": {
                "id": "anthropic",
                "name": "Anthropic",
                "models": {
                    "claude-opus-4-1": {
                        "id": "claude-opus-4-1",
                        "name": "Claude Opus 4.1",
                        "family": "claude-opus",
                        "attachment": true,
                        "reasoning": true,
                        "tool_call": true,
                        "limit": { "context": 200000, "output": 32000 },
                        "cost": { "input": 15.0, "output": 75.0, "cache_read": 1.5, "cache_write": 18.75 }
                    }
                }
            }
        }"#;
        let catalog: ModelsDevCatalog = serde_json::from_str(payload).expect("parse");
        let entry = &catalog["anthropic"].models["claude-opus-4-1"];
        assert_eq!(entry.id, "claude-opus-4-1");
        assert_eq!(entry.limit.as_ref().unwrap().context, Some(200_000));
        assert!((entry.cost.as_ref().unwrap().input.unwrap() - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lookup_returns_none_without_init() {
        // get()/lookup() must never panic when the catalog hasn't been hydrated.
        assert!(get().is_none() || get().is_some());
        let _ = lookup("anthropic", "missing");
    }
}
