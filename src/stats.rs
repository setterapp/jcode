use anyhow::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub total_messages: usize,
    pub total_tokens_input: u64,
    pub total_tokens_output: u64,
    pub total_tokens_cache: u64,
    pub total_cost_usd: f64,
    pub by_model: Vec<ModelStats>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelStats {
    pub model: String,
    pub sessions: usize,
    pub messages: usize,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub cost_usd: f64,
}

impl SessionStats {
    pub fn new() -> Self {
        Self {
            total_sessions: 0,
            total_messages: 0,
            total_tokens_input: 0,
            total_tokens_output: 0,
            total_tokens_cache: 0,
            total_cost_usd: 0.0,
            by_model: vec![],
        }
    }
}

pub fn load_stats() -> Result<SessionStats> {
    let flavor = crate::storage::product_flavor();
    let data_dir = dirs::data_dir()
        .map(|d| d.join(flavor.config_dir_name()))
        .unwrap_or_else(|| std::path::PathBuf::from(format!(
            "~/.local/share/{}",
            flavor.config_dir_name()
        )));

    let stats_file = data_dir.join("usage_stats.json");
    if !stats_file.exists() {
        return Ok(SessionStats::new());
    }

    let content = std::fs::read_to_string(&stats_file)?;
    let stats: SessionStats = serde_json::from_str(&content)?;
    Ok(stats)
}

pub fn save_stats(stats: &SessionStats) -> Result<()> {
    let flavor = crate::storage::product_flavor();
    let data_dir = dirs::data_dir()
        .map(|d| d.join(flavor.config_dir_name()))
        .unwrap_or_else(|| std::path::PathBuf::from(format!(
            "~/.local/share/{}",
            flavor.config_dir_name()
        )));
    std::fs::create_dir_all(&data_dir)?;

    let stats_file = data_dir.join("usage_stats.json");
    std::fs::write(&stats_file, serde_json::to_string_pretty(stats)?)?;
    Ok(())
}
