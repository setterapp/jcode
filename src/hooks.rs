//! Lightweight, opt-in hooks system that mirrors Claude Code's user-script
//! hook surface. Hooks are shell commands the user wires into specific
//! lifecycle events (session start/end, before/after a tool call). Each hook
//! runs in its own subprocess with the event payload piped to stdin as JSON.
//!
//! Configuration lives at `<JCODE_HOME>/hooks.toml`. Format:
//!
//! ```toml
//! [[hook]]
//! event = "session_start"
//! command = "echo started >> /tmp/jcode-hooks.log"
//!
//! [[hook]]
//! event = "post_tool_use"
//! tool = "bash"        # optional filter; matches tool name (case-insensitive)
//! command = "/usr/local/bin/notify.sh"
//! timeout_ms = 1500    # optional, defaults to 1500ms
//! ```
//!
//! Hooks are intentionally fire-and-forget. They never block the TUI: a slow
//! script gets SIGKILLed after `timeout_ms`. Failure to spawn is logged but
//! never raised to the user.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

/// Lifecycle events that user hooks can subscribe to. Names match Claude Code
/// where possible so existing scripts work unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    SessionStart,
    SessionEnd,
    PreToolUse,
    PostToolUse,
    UserPromptSubmit,
    Stop,
}

impl HookEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            HookEvent::SessionStart => "session_start",
            HookEvent::SessionEnd => "session_end",
            HookEvent::PreToolUse => "pre_tool_use",
            HookEvent::PostToolUse => "post_tool_use",
            HookEvent::UserPromptSubmit => "user_prompt_submit",
            HookEvent::Stop => "stop",
        }
    }
}

/// One hook entry from `hooks.toml`.
#[derive(Debug, Clone, Deserialize)]
struct HookEntry {
    event: HookEvent,
    command: String,
    /// Optional filter: only fire for tool calls whose name matches (case-insensitive).
    /// Ignored for non-tool events.
    #[serde(default)]
    tool: Option<String>,
    /// Hard timeout before the runner SIGKILLs the script. Defaults to 1500ms.
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct HooksFile {
    #[serde(default, rename = "hook")]
    hooks: Vec<HookEntry>,
}

/// Process-wide cache of parsed hook entries. Loaded lazily on first event.
/// Invalidated only by a process restart; users editing `hooks.toml` need to
/// relaunch (matches Claude Code's behavior).
static HOOKS: OnceLock<Vec<HookEntry>> = OnceLock::new();

fn hooks_path() -> Option<PathBuf> {
    crate::storage::jcode_dir().ok().map(|dir| dir.join("hooks.toml"))
}

fn load_hooks() -> Vec<HookEntry> {
    let Some(path) = hooks_path() else {
        return Vec::new();
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    match toml::from_str::<HooksFile>(&contents) {
        Ok(file) => file.hooks,
        Err(err) => {
            crate::logging::info(&format!("hooks.toml parse error: {}", err));
            Vec::new()
        }
    }
}

fn hooks() -> &'static [HookEntry] {
    HOOKS.get_or_init(load_hooks)
}

/// Event payload passed to user scripts on stdin as a single JSON object.
/// Stays minimal so additions don't break existing scripts.
#[derive(Debug, Serialize)]
pub struct HookPayload<'a> {
    pub event: &'a str,
    pub session_id: &'a str,
    pub cwd: String,
    /// Tool name, if this is a tool event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<&'a str>,
    /// User input, if this is a prompt event.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_prompt: Option<&'a str>,
    /// Tool input/output preview, if available. Truncated for safety.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_payload_preview: Option<String>,
}

impl<'a> HookPayload<'a> {
    pub fn new(event: HookEvent, session_id: &'a str) -> Self {
        Self {
            event: event.as_str(),
            session_id,
            cwd: std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            tool: None,
            user_prompt: None,
            tool_payload_preview: None,
        }
    }
}

/// Fire every hook subscribed to `event` whose optional `tool` filter
/// matches. Non-blocking — runner tasks are detached. Safe to call when no
/// tokio runtime is present (becomes a no-op).
pub fn dispatch(event: HookEvent, payload: &HookPayload<'_>) {
    let entries: Vec<HookEntry> = hooks()
        .iter()
        .filter(|h| h.event == event)
        .filter(|h| {
            let Some(filter) = h.tool.as_deref() else {
                return true;
            };
            payload
                .tool
                .map(|t| t.eq_ignore_ascii_case(filter))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    if entries.is_empty() {
        return;
    }

    let json = match serde_json::to_string(payload) {
        Ok(s) => s,
        Err(err) => {
            crate::logging::info(&format!("hooks: failed to serialize payload: {}", err));
            return;
        }
    };

    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };

    for entry in entries {
        let json = json.clone();
        handle.spawn(async move {
            let timeout =
                Duration::from_millis(entry.timeout_ms.unwrap_or(1500).clamp(50, 30_000));
            if let Err(err) = run_hook(&entry.command, &json, timeout).await {
                crate::logging::info(&format!(
                    "hook `{}` failed for {}: {}",
                    entry.command,
                    entry.event.as_str(),
                    err
                ));
            }
        });
    }
}

async fn run_hook(command: &str, payload: &str, timeout: Duration) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;
    use tokio::process::Command;

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        let bytes = payload.as_bytes().to_vec();
        tokio::spawn(async move {
            let _ = stdin.write_all(&bytes).await;
            let _ = stdin.shutdown().await;
        });
    }

    let waiter = child.wait_with_output();
    match tokio::time::timeout(timeout, waiter).await {
        Ok(Ok(_output)) => Ok(()),
        Ok(Err(err)) => Err(err.into()),
        Err(_) => anyhow::bail!("hook timed out after {:?}", timeout),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_hook_entries() {
        let toml_str = r#"
[[hook]]
event = "session_start"
command = "echo a"

[[hook]]
event = "post_tool_use"
tool = "bash"
command = "echo b"
timeout_ms = 500
"#;
        let parsed: HooksFile = toml::from_str(toml_str).expect("parse");
        assert_eq!(parsed.hooks.len(), 2);
        assert_eq!(parsed.hooks[0].event, HookEvent::SessionStart);
        assert_eq!(parsed.hooks[1].event, HookEvent::PostToolUse);
        assert_eq!(parsed.hooks[1].tool.as_deref(), Some("bash"));
    }
}
