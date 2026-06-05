use super::{BackgroundInfo, InfoWidgetData, SwarmInfo, truncate_smart};
use crate::protocol::SwarmMemberStatus;
use crate::tui::color_support::rgb;
use ratatui::prelude::*;

pub(super) fn render_swarm_widget(data: &InfoWidgetData, inner: Rect) -> Vec<Line<'static>> {
    let Some(info) = &data.swarm_info else {
        return Vec::new();
    };

    let mut lines: Vec<Line> = vec![render_swarm_stats_line(info)];

    if info.members.is_empty()
        && let Some(status) = &info.subagent_status
    {
        lines.push(Line::from(vec![
            Span::styled("▶ ", Style::default().fg(rgb(255, 200, 100))),
            Span::styled(
                truncate_smart(status, inner.width.saturating_sub(4) as usize),
                Style::default().fg(rgb(200, 200, 210)),
            ),
        ]));
    }

    let max_names = inner.height.saturating_sub(lines.len() as u16) as usize;
    let max_name_len = inner.width.saturating_sub(6) as usize;
    if !info.members.is_empty() {
        for member in info.members.iter().take(max_names.min(3)) {
            lines.push(swarm_member_line(member, max_name_len));
        }
    } else {
        for name in info.session_names.iter().take(max_names.min(3)) {
            lines.push(render_swarm_name_line(name, max_name_len));
        }
    }

    lines
}

pub(super) fn render_background_widget(data: &InfoWidgetData, inner: Rect) -> Vec<Line<'static>> {
    let Some(info) = &data.background_info else {
        return Vec::new();
    };

    render_background_lines(info, inner.width as usize)
}

pub(super) fn render_background_compact(info: &BackgroundInfo) -> Vec<Line<'static>> {
    render_background_lines(info, 40)
}

fn swarm_member_label(member: &SwarmMemberStatus) -> String {
    member
        .friendly_name
        .clone()
        .unwrap_or_else(|| member.session_id.chars().take(8).collect())
}

fn swarm_status_style(status: &str) -> (Color, &'static str) {
    match status {
        "spawned" => (rgb(140, 140, 150), "○"),
        "ready" => (rgb(120, 180, 120), "●"),
        "running" => (rgb(255, 200, 100), "▶"),
        "blocked" => (rgb(255, 170, 80), "⏸"),
        "failed" => (rgb(255, 100, 100), "✗"),
        "completed" => (rgb(100, 200, 100), "✓"),
        "stopped" => (rgb(140, 140, 150), "■"),
        "crashed" => (rgb(255, 80, 80), "!"),
        _ => (rgb(140, 140, 150), "·"),
    }
}

fn swarm_role_prefix(member: &SwarmMemberStatus) -> &'static str {
    match member.role.as_deref() {
        Some("coordinator") => "★ ",
        Some("worktree_manager") => "◆ ",
        _ => "  ",
    }
}

/// Stale threshold: a running member that hasn't changed status in this many
/// seconds is considered possibly stuck and highlighted in orange.
const SWARM_STALE_SECS: u64 = 45;

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Shorten a model id for badge display: drop provider prefix and version noise.
fn model_short(model: &str) -> String {
    model.rsplit('/').next().unwrap_or(model).to_string()
}

/// Format elapsed millis as `m:ss` (or `h:mm:ss` past an hour).
fn format_elapsed(started_at_unix_ms: u64) -> String {
    let now = now_unix_ms();
    let secs = now.saturating_sub(started_at_unix_ms) / 1000;
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}

fn member_is_stale(member: &SwarmMemberStatus) -> bool {
    member.status == "running"
        && member
            .status_age_secs
            .map(|s| s > SWARM_STALE_SECS)
            .unwrap_or(false)
}

fn swarm_member_line(member: &SwarmMemberStatus, max_width: usize) -> Line<'static> {
    let name = swarm_member_label(member);
    let role_prefix = swarm_role_prefix(member);
    let (status_color, icon) = swarm_status_style(&member.status);
    let stale = member_is_stale(member);

    let mut spans: Vec<Span> = vec![
        Span::styled(role_prefix.to_string(), Style::default().fg(rgb(255, 200, 100))),
        Span::styled(format!("{} ", icon), Style::default().fg(status_color)),
        Span::styled(name, Style::default().fg(rgb(210, 210, 220))),
    ];

    // Model badge (pink), distinct from the action text.
    if let Some(model) = member.model.as_deref() {
        spans.push(Span::styled(
            format!("  {}", model_short(model)),
            Style::default().fg(rgb(238, 153, 200)),
        ));
    }

    // Action / current detail (dim), space-budgeted against the row width.
    // Reserve room for the trailing elapsed time (~9) and stale marker (~2).
    if let Some(detail) = member.detail.as_deref().filter(|d| !d.is_empty()) {
        let used = name_len(member) + 4 + 12;
        let budget = max_width.saturating_sub(used).max(8);
        spans.push(Span::styled(
            format!("  {}", truncate_smart(detail, budget)),
            Style::default().fg(rgb(140, 140, 150)),
        ));
    }

    // Elapsed time (cyan, or orange when stale), plus a stale marker.
    if let Some(started) = member.started_at_unix_ms {
        let elapsed_color = if stale { rgb(220, 160, 60) } else { rgb(120, 175, 230) };
        spans.push(Span::styled(
            format!("  {}", format_elapsed(started)),
            Style::default().fg(elapsed_color),
        ));
    }
    if stale {
        spans.push(Span::styled(" ⚠", Style::default().fg(rgb(220, 160, 60))));
    }

    Line::from(spans)
}

fn name_len(member: &SwarmMemberStatus) -> usize {
    swarm_member_label(member).chars().count()
        + member.model.as_deref().map(|m| model_short(m).chars().count() + 2).unwrap_or(0)
}

fn render_swarm_stats_line(info: &SwarmInfo) -> Line<'static> {
    let mut stats_parts: Vec<Span> =
        vec![Span::styled("🐝 ", Style::default().fg(rgb(255, 200, 100)))];

    if info.session_count > 0 {
        stats_parts.push(Span::styled(
            format!("{}s", info.session_count),
            Style::default().fg(rgb(160, 160, 170)),
        ));
    }
    if let Some(clients) = info.client_count {
        if info.session_count > 0 {
            stats_parts.push(Span::styled(" · ", Style::default().fg(rgb(100, 100, 110))));
        }
        stats_parts.push(Span::styled(
            format!("{}c", clients),
            Style::default().fg(rgb(160, 160, 170)),
        ));
    }

    Line::from(stats_parts)
}

fn render_swarm_name_line(name: &str, max_name_len: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled("  · ", Style::default().fg(rgb(100, 100, 110))),
        Span::styled(
            truncate_smart(name, max_name_len),
            Style::default().fg(rgb(140, 140, 150)),
        ),
    ])
}

fn render_background_lines(info: &BackgroundInfo, width: usize) -> Vec<Line<'static>> {
    let Some(summary) = background_summary(info) else {
        return Vec::new();
    };
    let mut lines = vec![Line::from(vec![
        Span::styled("⏳ ", Style::default().fg(rgb(180, 140, 255))),
        Span::styled(summary, Style::default().fg(rgb(160, 160, 170))),
    ])];

    let row_width = width.saturating_sub(4).max(12);
    for (index, task) in info.running_tasks.iter().take(3).enumerate() {
        let detail = if index == 0 {
            info.progress_detail.as_deref()
        } else {
            None
        };
        let row_text = if let Some(detail) = detail {
            truncate_smart(&format!("{} · {}", task, detail), row_width)
        } else {
            truncate_smart(task, row_width)
        };
        lines.push(Line::from(vec![
            Span::styled("  • ", Style::default().fg(rgb(120, 120, 130))),
            Span::styled(row_text, Style::default().fg(rgb(180, 180, 190))),
        ]));
    }

    let hidden = info.running_tasks.len().saturating_sub(3);
    if hidden > 0 {
        lines.push(Line::from(vec![
            Span::styled("   ", Style::default().fg(rgb(100, 100, 110))),
            Span::styled(
                format!("+{} more", hidden),
                Style::default().fg(rgb(140, 140, 150)),
            ),
        ]));
    }

    lines
}

fn background_summary(info: &BackgroundInfo) -> Option<String> {
    if info.running_count == 0 {
        return None;
    }

    Some(format!("Background · {} running", info.running_count))
}
