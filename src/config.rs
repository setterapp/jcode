//! Configuration file support for jcode
//!
//! Config is loaded from `~/.jcode/config.toml` (or `$JCODE_HOME/config.toml`)
//! Environment variables override config file settings.

pub use jcode_config_types::{
    AgentsConfig, AmbientConfig, AuthConfig, AutoJudgeConfig, AutoReviewConfig, CompactionConfig,
    CompactionMode, CrossProviderFailoverMode, DiagramDisplayMode, DiagramPanePosition,
    DiffDisplayMode, DisplayConfig, FeatureConfig, GatewayConfig, KeybindingsConfig,
    MarkdownSpacingMode, NamedProviderAuth, NamedProviderConfig, NamedProviderModelConfig,
    NamedProviderType, NativeScrollbarConfig, ProviderConfig, SafetyConfig,
    SessionPickerResumeAction, TaskModelsConfig, UpdateChannel,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Get the global config instance (loaded once on first access)
pub fn config() -> &'static Config {
    CONFIG.get_or_init(Config::load)
}

/// Main configuration struct
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// Keybinding configuration
    pub keybindings: KeybindingsConfig,

    /// External dictation / speech-to-text integration
    pub dictation: DictationConfig,

    /// Display/UI configuration
    pub display: DisplayConfig,

    /// Feature toggles
    pub features: FeatureConfig,

    /// Auth trust / consent configuration
    pub auth: AuthConfig,

    /// Provider configuration
    pub provider: ProviderConfig,

    /// Named provider profiles, keyed by profile name.
    ///
    /// Example:
    /// [providers.my-gateway]
    /// type = "openai-compatible"
    /// base_url = "https://llm.example.com/v1"
    /// api_key_env = "MY_GATEWAY_API_KEY"
    pub providers: BTreeMap<String, NamedProviderConfig>,

    /// Agent-specific model defaults
    pub agents: AgentsConfig,

    /// Ambient mode configuration
    pub ambient: AmbientConfig,

    /// Safety / notification configuration
    pub safety: SafetyConfig,

    /// WebSocket gateway configuration (for iOS/web clients)
    pub gateway: GatewayConfig,

    /// Compaction configuration
    pub compaction: CompactionConfig,

    /// Auto-review configuration
    pub autoreview: AutoReviewConfig,

    /// Auto-judge configuration
    pub autojudge: AutoJudgeConfig,

    /// Custom bottom status bar driven by a user shell script.
    /// When enabled, the runner spawns the command every `interval_ms`,
    /// passes session context as JSON via stdin, and renders stdout
    /// (with ANSI color escapes) at the bottom of the TUI.
    /// Mirrors the `statusLine` feature in Claude Code so existing
    /// scripts (e.g. `~/.claude/statusline-command.sh`) work verbatim.
    pub status_line: StatusLineConfig,

    /// Per-task-type model assignment. Slash commands like `/plan` and
    /// `/code` swap the active model for one turn based on entries here.
    pub task_models: TaskModelsConfig,
}

/// External dictation / speech-to-text integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DictationConfig {
    /// Shell command to run. Must print the transcript to stdout.
    pub command: String,
    /// How to apply the resulting transcript.
    pub mode: crate::protocol::TranscriptMode,
    /// Optional in-app hotkey to trigger dictation.
    pub key: String,
    /// Maximum time to wait for the command to finish (0 = no timeout).
    pub timeout_secs: u64,
}

impl Default for DictationConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            mode: crate::protocol::TranscriptMode::Send,
            key: "off".to_string(),
            timeout_secs: 90,
        }
    }
}

/// Custom bottom status bar driven by a user shell script.
///
/// When `enabled` is true and `command` is set, jcode spawns the command
/// every `interval_ms` milliseconds, passes session context as JSON via
/// stdin, and renders stdout (ANSI escape codes interpreted) at the bottom
/// of the TUI. JSON schema is a superset of Claude Code's `statusLine` hook
/// payload so existing scripts work without modification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StatusLineConfig {
    /// Whether the hook is active. When false, the bottom bar falls back to
    /// the built-in provider auth strip.
    pub enabled: bool,
    /// Shell command to execute (e.g. `bash ~/.jcode/statusline.sh`).
    pub command: Option<String>,
    /// How often to invoke the command. Clamped to `[200, 60_000]` ms.
    pub interval_ms: u64,
    /// Maximum time to wait for the command to finish before SIGKILL.
    pub timeout_ms: u64,
}

impl Default for StatusLineConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            command: None,
            interval_ms: 1000,
            timeout_ms: 500,
        }
    }
}

impl StatusLineConfig {
    /// Returns `interval_ms` clamped to a sensible range so a misconfigured
    /// value can't melt the user's CPU or visually freeze for minutes.
    pub fn effective_interval_ms(&self) -> u64 {
        self.interval_ms.clamp(200, 60_000)
    }

    /// Returns `timeout_ms` clamped so the runner can't be blocked by a
    /// runaway script.
    pub fn effective_timeout_ms(&self) -> u64 {
        self.timeout_ms.clamp(50, 10_000)
    }

    /// True iff the hook is configured and should run.
    pub fn is_active(&self) -> bool {
        self.enabled && self.command.as_deref().map(str::trim).is_some_and(|c| !c.is_empty())
    }
}

mod config_file;
mod default_file;
mod display_summary;
mod env_overrides;

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
