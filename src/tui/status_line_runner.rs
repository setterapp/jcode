//! User-defined status-line hook runner.
//!
//! Mirrors Claude Code's `statusLine.command` mechanism: a user shell script
//! is invoked at a fixed cadence with session context as JSON via stdin and
//! its stdout (with ANSI color escapes) is rendered as the bottom bar of the
//! TUI.
//!
//! The runner is a single tokio task. It owns no mutable TUI state — it
//! reads the current snapshot from a global cell (updated by the TUI on
//! each frame) and writes the result back into a global cell that the
//! renderer reads on every draw.
//!
//! Design notes:
//! - Single global runner: starting twice is a no-op.
//! - In-flight guard: a script that runs longer than `interval_ms` won't
//!   overlap; the next tick is skipped if a previous invocation is still
//!   alive (`MissedTickBehavior::Skip`).
//! - Hard timeout: the runner SIGKILLs scripts that exceed `timeout_ms`.
//! - Bus broadcast: when output changes, publishes `StatusLineUpdated` so
//!   the TUI redraws without polling.

use crate::bus::{Bus, BusEvent};
use crate::config::StatusLineConfig;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::oneshot;
use tokio::time::MissedTickBehavior;

/// Snapshot of session state passed to the user shell script as JSON via
/// stdin. The schema is a SUPERSET of Claude Code's `statusLine` payload so
/// existing scripts work without modification.
#[derive(Debug, Clone, Default)]
pub struct StatusLineSnapshot {
    pub model_id: String,
    pub model_display: String,
    pub cwd: PathBuf,
    pub branch: Option<String>,
    pub total_cost_usd: f32,
    pub context_used_percentage: f64,
    pub context_window_size: u64,
    pub current_usage_tokens: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    /// Seconds remaining until 5h rate limit resets.
    pub five_hour_resets_in_secs: Option<i64>,
    /// Seconds remaining until 7d rate limit resets.
    pub seven_day_resets_in_secs: Option<i64>,
}

#[derive(Serialize)]
struct ScriptInputPayload<'a> {
    model: ModelPayload<'a>,
    workspace: WorkspacePayload<'a>,
    cost: CostPayload,
    context_window: ContextPayload,
    #[serde(skip_serializing_if = "Option::is_none")]
    rate_limits: Option<RateLimitsPayload>,
    /// Schema marker so scripts can branch on shape if Claude Code diverges.
    schema_version: u32,
    /// Tool emitting the payload, lets shared scripts target jcode-only fields.
    source: &'static str,
}

#[derive(Serialize)]
struct ModelPayload<'a> {
    id: &'a str,
    display_name: &'a str,
}

#[derive(Serialize)]
struct WorkspacePayload<'a> {
    current_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<&'a str>,
}

#[derive(Serialize)]
struct CostPayload {
    total_cost_usd: f32,
}

#[derive(Serialize)]
struct ContextPayload {
    total_input_tokens: u64,
    total_output_tokens: u64,
    context_window_size: u64,
    current_usage: u64,
    used_percentage: f64,
}

#[derive(Serialize)]
struct RateLimitsPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    five_hour: Option<RateBucketPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seven_day: Option<RateBucketPayload>,
}

#[derive(Serialize)]
struct RateBucketPayload {
    /// Unix timestamp (seconds) when the bucket resets.
    resets_at: i64,
}

fn snapshot_cell() -> &'static Mutex<StatusLineSnapshot> {
    static CELL: OnceLock<Mutex<StatusLineSnapshot>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(StatusLineSnapshot::default()))
}

fn output_cell() -> &'static Mutex<Option<String>> {
    static CELL: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(None))
}

fn started_flag() -> &'static Mutex<bool> {
    static CELL: OnceLock<Mutex<bool>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(false))
}

/// Update the snapshot the runner will pass to the script on the next tick.
/// Cheap to call from the render path (single mutex write on a small struct).
pub fn set_snapshot(snapshot: StatusLineSnapshot) {
    if let Ok(mut guard) = snapshot_cell().lock() {
        *guard = snapshot;
    }
}

/// Latest stdout from the user script. `None` until the first successful run.
pub fn current_output() -> Option<String> {
    output_cell().lock().ok().and_then(|g| g.clone())
}

/// Launch the runner task. Idempotent — second call is a no-op. Returns
/// immediately; the actual work happens on a detached tokio task.
pub fn spawn_runner(config: StatusLineConfig) {
    crate::logging::info(&format!(
        "status_line: spawn_runner called (enabled={}, has_command={})",
        config.enabled,
        config.command.is_some()
    ));
    if !config.is_active() {
        crate::logging::info("status_line: not active, skipping spawn");
        return;
    }
    {
        let mut started = match started_flag().lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if *started {
            crate::logging::info("status_line: already started, skipping");
            return;
        }
        *started = true;
    }

    let interval_ms = config.effective_interval_ms();
    let timeout_ms = config.effective_timeout_ms();
    let command = match config.command {
        Some(cmd) => cmd,
        None => return,
    };
    crate::logging::info(&format!(
        "status_line: starting runner interval={}ms timeout={}ms cmd={}",
        interval_ms, timeout_ms, command
    ));

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(interval_ms));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        // Skip the first immediate tick — let the TUI render at least one
        // frame so the snapshot is populated.
        interval.tick().await;

        loop {
            interval.tick().await;
            let snapshot = snapshot_cell()
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default();
            let json_input = match build_payload_json(&snapshot) {
                Ok(s) => s,
                Err(err) => {
                    crate::logging::info(&format!(
                        "status_line: failed to serialize snapshot: {}",
                        err
                    ));
                    continue;
                }
            };

            match run_script(&command, &json_input, timeout_ms).await {
                Ok(stdout) => {
                    let len = stdout.len();
                    let changed = {
                        match output_cell().lock() {
                            Ok(mut guard) => {
                                let prev = guard.as_deref();
                                let new = stdout.as_str();
                                if prev != Some(new) {
                                    *guard = Some(stdout.clone());
                                    true
                                } else {
                                    false
                                }
                            }
                            Err(_) => false,
                        }
                    };
                    if changed {
                        crate::logging::info(&format!(
                            "status_line: stdout changed ({} bytes), publishing redraw",
                            len
                        ));
                        Bus::global().publish(BusEvent::StatusLineUpdated);
                    }
                }
                Err(err) => {
                    crate::logging::info(&format!(
                        "status_line: hook failed: {}",
                        err
                    ));
                }
            }
        }
    });
}

fn build_payload_json(snapshot: &StatusLineSnapshot) -> Result<String, serde_json::Error> {
    let now_secs: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let rate_limits =
        if snapshot.five_hour_resets_in_secs.is_some() || snapshot.seven_day_resets_in_secs.is_some() {
            Some(RateLimitsPayload {
                five_hour: snapshot
                    .five_hour_resets_in_secs
                    .map(|s| RateBucketPayload { resets_at: now_secs + s }),
                seven_day: snapshot
                    .seven_day_resets_in_secs
                    .map(|s| RateBucketPayload { resets_at: now_secs + s }),
            })
        } else {
            None
        };

    let payload = ScriptInputPayload {
        model: ModelPayload {
            id: &snapshot.model_id,
            display_name: &snapshot.model_display,
        },
        workspace: WorkspacePayload {
            current_dir: snapshot.cwd.to_string_lossy().into_owned(),
            branch: snapshot.branch.as_deref(),
        },
        cost: CostPayload {
            total_cost_usd: snapshot.total_cost_usd,
        },
        context_window: ContextPayload {
            total_input_tokens: snapshot.total_input_tokens,
            total_output_tokens: snapshot.total_output_tokens,
            context_window_size: snapshot.context_window_size,
            current_usage: snapshot.current_usage_tokens,
            used_percentage: snapshot.context_used_percentage,
        },
        rate_limits,
        schema_version: 1,
        source: "jcode",
    };

    serde_json::to_string(&payload)
}

async fn run_script(
    command: &str,
    stdin_payload: &str,
    timeout_ms: u64,
) -> anyhow::Result<String> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload_bytes = stdin_payload.as_bytes().to_vec();
        // Write stdin in a sub-task so a script that doesn't read stdin can
        // still be killed by our timeout below.
        let _ = tokio::spawn(async move {
            let _ = stdin.write_all(&payload_bytes).await;
            let _ = stdin.shutdown().await;
        });
    }

    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
        let _ = tx.send(());
    });

    tokio::select! {
        out = child.wait_with_output() => {
            let out = out?;
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            Ok(stdout)
        }
        _ = rx => {
            anyhow::bail!("status_line script timed out after {}ms", timeout_ms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_payload_emits_minimal_valid_json() {
        let snap = StatusLineSnapshot {
            model_id: "claude-sonnet-4-6".into(),
            model_display: "Sonnet 4.6".into(),
            cwd: PathBuf::from("/tmp"),
            branch: Some("main".into()),
            total_cost_usd: 0.42,
            context_used_percentage: 12.5,
            context_window_size: 200_000,
            current_usage_tokens: 25_000,
            total_input_tokens: 20_000,
            total_output_tokens: 5_000,
            five_hour_resets_in_secs: None,
            seven_day_resets_in_secs: None,
        };
        let s = build_payload_json(&snap).expect("serialize");
        assert!(s.contains("\"id\":\"claude-sonnet-4-6\""), "{s}");
        assert!(s.contains("\"display_name\":\"Sonnet 4.6\""), "{s}");
        assert!(s.contains("\"current_dir\":\"/tmp\""), "{s}");
        assert!(s.contains("\"branch\":\"main\""), "{s}");
        assert!(s.contains("\"total_cost_usd\":0.42"), "{s}");
        assert!(s.contains("\"used_percentage\":12.5"), "{s}");
        assert!(s.contains("\"schema_version\":1"), "{s}");
        assert!(s.contains("\"source\":\"jcode\""), "{s}");
        // No rate_limits key when both are None
        assert!(!s.contains("rate_limits"), "{s}");
    }

    #[test]
    fn build_payload_includes_rate_limits_when_present() {
        let snap = StatusLineSnapshot {
            five_hour_resets_in_secs: Some(3_600),
            seven_day_resets_in_secs: Some(86_400 * 4),
            ..Default::default()
        };
        let s = build_payload_json(&snap).expect("serialize");
        assert!(s.contains("rate_limits"), "{s}");
        assert!(s.contains("five_hour"), "{s}");
        assert!(s.contains("seven_day"), "{s}");
    }

    #[tokio::test]
    async fn run_script_captures_stdout_with_stdin_payload() {
        // jq isn't always installed; use bash that just echoes the payload.
        let out =
            run_script("cat", "{\"model\":{\"id\":\"gpt-5\"}}", 2000)
                .await
                .expect("script");
        assert!(out.contains("gpt-5"), "{out}");
    }

    #[tokio::test]
    async fn run_script_times_out_long_runner() {
        let err = run_script("sleep 5", "", 100).await.expect_err("should time out");
        assert!(err.to_string().contains("timed out"), "err: {err}");
    }
}
