use super::{App, DisplayMessage, PlanModeState};
use crate::message::{ContentBlock, Role};
use std::path::PathBuf;

/// Tools that mutate files, git, or spawn worker agents. Blocked while `/plan`
/// read-only planning mode is active so the agent can only inspect and plan.
const PLAN_BLOCKED_TOOLS: &[&str] = &[
    "edit",
    "write",
    "multiedit",
    "apply_patch",
    "patch",
    "side_panel",
    "memory",
    "mcp",
    "bg",
    "subagent",
    "batch",
];

#[derive(Debug, Clone)]
pub(super) enum PlanCommand {
    /// Enter plan mode. `prompt` is an optional planning request to submit now.
    Enter { prompt: Option<String> },
    /// Approve the current plan: persist it and exit plan mode to execute.
    Approve,
    /// Leave plan mode without executing.
    Off,
    /// Show whether plan mode is active.
    Status,
}

/// Parse a `/plan ...` command. Returns `None` for non-`/plan` input so the
/// dispatch chain falls through to other handlers.
pub(super) fn parse_plan_command(trimmed: &str) -> Option<Result<PlanCommand, String>> {
    let rest = trimmed.strip_prefix("/plan")?;
    // Require either end-of-token or whitespace so `/planfoo` is not matched.
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return Some(Ok(PlanCommand::Enter { prompt: None }));
    }
    match rest {
        "approve" | "apply" | "go" => return Some(Ok(PlanCommand::Approve)),
        "off" | "stop" | "cancel" | "exit" => return Some(Ok(PlanCommand::Off)),
        "status" => return Some(Ok(PlanCommand::Status)),
        _ => {}
    }
    Some(Ok(PlanCommand::Enter {
        prompt: Some(rest.to_string()),
    }))
}

pub(super) fn handle_plan_command_local(app: &mut App, command: PlanCommand) {
    match command {
        PlanCommand::Enter { prompt } => enter_plan_mode(app, prompt),
        PlanCommand::Approve => approve_plan(app),
        PlanCommand::Off => exit_plan_mode(app, true),
        PlanCommand::Status => {
            let msg = match app.plan_mode.as_ref() {
                Some(state) => format!(
                    "Plan mode ACTIVE (read-only). Approved plans write to {}. Use /plan approve to execute, /plan off to cancel.",
                    state.plan_file.display()
                ),
                None => "Plan mode is inactive. Use /plan [what to plan] to start.".to_string(),
            };
            app.push_display_message(DisplayMessage::system(msg));
        }
    }
}

fn enter_plan_mode(app: &mut App, prompt: Option<String>) {
    let plan_file = plan_file_path(app);
    app.plan_mode = Some(PlanModeState {
        plan_file: plan_file.clone(),
        entry_msg_count: app.session.messages.len(),
    });
    app.push_display_message(DisplayMessage::system(format!(
        "◆ Plan mode ON — read-only. File edits, patches, subagents, and MCP writes are blocked \
(read-only shell still works — avoid mutating commands). Present a plan; run /plan approve to \
execute (writes {}), or /plan off to cancel.",
        plan_file.display()
    )));
    app.set_status_notice("plan mode on");

    // The read-only guardrail is injected into every turn while plan mode is
    // active (see App::append_plan_mode_guardrail), so just submit the request.
    if let Some(prompt) = prompt {
        submit_prompt(app, prompt);
    }
}

fn approve_plan(app: &mut App) {
    let Some((plan_file, entry_msg_count)) = app
        .plan_mode
        .as_ref()
        .map(|s| (s.plan_file.clone(), s.entry_msg_count))
    else {
        app.push_display_message(DisplayMessage::system(
            "Not in plan mode. Use /plan [what to plan] first.".to_string(),
        ));
        return;
    };

    match write_plan_file(app, &plan_file, entry_msg_count) {
        Ok(true) => app.push_display_message(DisplayMessage::system(format!(
            "✓ Plan approved → {}. Executing.",
            plan_file.display()
        ))),
        Ok(false) => app.push_display_message(DisplayMessage::system(
            "✓ Plan approved. Executing. (No plan text captured to file yet.)".to_string(),
        )),
        Err(err) => app.push_display_message(DisplayMessage::system(format!(
            "✓ Plan approved. Executing. (Could not write plan file: {})",
            err
        ))),
    }

    app.plan_mode = None;
    submit_prompt(
        app,
        "The plan is approved. Execute it now: make the changes and run any needed checks."
            .to_string(),
    );
}

fn exit_plan_mode(app: &mut App, announce: bool) {
    if app.plan_mode.take().is_some() {
        if announce {
            app.push_display_message(DisplayMessage::system(
                "Plan mode off. Tools are no longer restricted.".to_string(),
            ));
            app.set_status_notice("plan mode off");
        }
    } else if announce {
        app.push_display_message(DisplayMessage::system(
            "Plan mode is already inactive.".to_string(),
        ));
    }
}

/// Submit `body` as a fresh user turn via the normal input path. `body` must not
/// begin with `/` so no slash handler reclaims it.
fn submit_prompt(app: &mut App, body: String) {
    app.input = body;
    app.submit_input();
}

pub(super) fn plan_guardrail() -> &'static str {
    "You are in PLAN MODE (read-only). The edit, write, multiedit, apply_patch, patch, \
side_panel, memory, mcp, bg, subagent, and batch tools are BLOCKED and will return an error \
if called. You may read, search, and run read-only shell commands — do NOT run mutating shell \
commands (rm, mv, writing files, git commit/push/reset, package installs). Inspect the codebase, \
then present a concise, actionable plan as your reply. The user runs /plan approve to execute it."
}

fn plan_file_path(app: &App) -> PathBuf {
    let base = app
        .session
        .working_dir
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    base.join(".jcode").join("plans").join(format!("plan-{}.md", stamp))
}

/// Write the most recent assistant message (the plan) to `path`, considering
/// only messages produced after plan mode was entered (`entry_msg_count`).
/// Returns Ok(true) if plan text was found and written, Ok(false) if there was
/// no assistant text to capture.
fn write_plan_file(app: &App, path: &PathBuf, entry_msg_count: usize) -> std::io::Result<bool> {
    let Some(text) = last_assistant_text(app, entry_msg_count) else {
        return Ok(false);
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(true)
}

fn last_assistant_text(app: &App, entry_msg_count: usize) -> Option<String> {
    let start = entry_msg_count.min(app.session.messages.len());
    for msg in app.session.messages[start..].iter().rev() {
        if msg.role != Role::Assistant {
            continue;
        }
        let mut text = String::new();
        for block in &msg.content {
            if let ContentBlock::Text { text: t, .. } = block {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(t);
            }
        }
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    None
}

impl App {
    /// While plan mode is active, append the read-only guardrail to every turn's
    /// system prompt so the model knows which tools are restricted.
    pub(super) fn append_plan_mode_guardrail(
        &self,
        split: &mut crate::prompt::SplitSystemPrompt,
    ) {
        if self.plan_mode.is_none() {
            return;
        }
        if !split.dynamic_part.is_empty() {
            split.dynamic_part.push_str("\n\n");
        }
        split.dynamic_part.push_str("# Plan Mode\n\n");
        split.dynamic_part.push_str(plan_guardrail());
    }

    /// If plan mode is active and `tool_name` is a mutating tool, return a block
    /// message to surface as the tool result instead of executing it.
    pub(super) fn plan_mode_block(&self, tool_name: &str) -> Option<String> {
        if self.plan_mode.is_none() {
            return None;
        }
        if PLAN_BLOCKED_TOOLS.contains(&tool_name) {
            Some(format!(
                "⊘ Plan mode is read-only — the '{}' tool is blocked. Present your plan as a \
message; the user runs /plan approve to execute it.",
                tool_name
            ))
        } else {
            None
        }
    }
}
