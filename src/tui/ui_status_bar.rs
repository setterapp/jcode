use crate::auth::{AuthState, AuthStatus};
use crate::tui::info_widget::{InfoWidgetData, UsageProvider};
use crate::tui::TuiState;
use ratatui::prelude::*;

pub(super) fn build_status_bar_line(
    app: &dyn TuiState,
    widget_data: &InfoWidgetData,
    width: usize,
) -> Line<'static> {
    let bg = Color::Reset;
    let dim = Color::Rgb(110, 110, 110);
    let accent = Color::Rgb(130, 180, 130);
    let warn = Color::Rgb(220, 160, 60);
    let sep_style = Style::default().fg(dim).bg(bg);
    let label_style = Style::default().fg(dim).bg(bg);
    let value_style = Style::default().fg(Color::Rgb(200, 200, 200)).bg(bg);
    let model_style = Style::default().fg(accent).bg(bg);

    let sep = Span::styled(" │ ", sep_style);

    let mut spans: Vec<Span<'static>> = Vec::new();

    // Rate limit warning (leftmost when active)
    if let Some(remaining) = app.rate_limit_remaining() {
        let secs = remaining.as_secs();
        spans.push(Span::styled("⚠ rate:", Style::default().fg(warn).bg(bg)));
        spans.push(Span::styled(
            format!("{}s", secs),
            Style::default().fg(warn).bg(bg),
        ));
        spans.push(sep.clone());
    }

    // Provider auth dots (before model name)
    let auth_spans = build_auth_dots(&app.auth_status(), bg);
    if !auth_spans.is_empty() {
        spans.extend(auth_spans);
        spans.push(sep.clone());
    }

    // Profile + model name
    let model_raw = app.provider_model();
    let model = jcode_client_core::strip_provider_prefix(&model_raw).to_string();
    let model_display = if model.is_empty() || model == "unknown" {
        "no model".to_string()
    } else {
        shorten_model(&model)
    };
    if let Some(profile) = app.active_profile() {
        let profile_style = Style::default().fg(Color::Rgb(140, 160, 220)).bg(bg);
        spans.push(Span::styled(format!("[{}] ", profile), profile_style));
    }
    spans.push(Span::styled(model_display, model_style));

    // Context window — visual bar + percentage (Claude Code style)
    if let (Some(limit), Some((input, _))) = (app.context_limit(), app.total_session_tokens()) {
        if limit > 0 {
            let ratio = (input as f64 / limit as f64).min(1.0);
            let pct = (ratio * 100.0) as u32;
            let bar_width = 10usize;
            let filled = ((ratio * bar_width as f64).round() as usize).min(bar_width);
            let empty = bar_width - filled;
            let bar_color = if pct >= 80 {
                warn
            } else {
                Color::Rgb(100, 200, 100)
            };
            let pct_color = if pct >= 80 {
                warn
            } else {
                Color::Rgb(160, 160, 160)
            };
            let bar_str = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
            spans.push(sep.clone());
            spans.push(Span::styled(bar_str, Style::default().fg(bar_color).bg(bg)));
            spans.push(Span::styled(
                format!(" {}%", pct),
                Style::default().fg(pct_color).bg(bg),
            ));
        }
    }

    // Token counts ↑in ↓out
    if let Some((input, output)) = app.total_session_tokens() {
        if input > 0 || output > 0 {
            spans.push(sep.clone());
            spans.push(Span::styled(
                format!("↑{}k ↓{}k", input / 1000, output / 1000),
                value_style,
            ));
        }
    }

    // 5h / 7d usage with time-to-reset (Claude Code style: "5h 56% → 2h 43m")
    if let Some(usage) = widget_data.usage_info.as_ref() {
        let is_rate_limit_provider = matches!(
            usage.provider,
            UsageProvider::Anthropic | UsageProvider::OpenAI | UsageProvider::Copilot
        );

        if is_rate_limit_provider {
            let five_pct = (usage.five_hour * 100.0) as u32;
            let seven_pct = (usage.seven_day * 100.0) as u32;

            // Show if we have any data or the provider is configured
            if five_pct > 0 || seven_pct > 0 || usage.available {
                let tick = app.workspace_animation_tick();
                let five_color = five_hour_color(five_pct, warn, tick);
                let seven_color = seven_day_band_color(seven_pct, warn);

                spans.push(sep.clone());
                spans.push(Span::styled("5h ", label_style.clone()));
                spans.push(Span::styled(
                    format!("{}%", five_pct),
                    Style::default().fg(five_color).bg(bg),
                ));
                if let Some(reset) = usage
                    .five_hour_resets_at
                    .as_deref()
                    .map(crate::usage::format_reset_time)
                {
                    spans.push(Span::styled(
                        format!(" → {}", reset),
                        Style::default().fg(dim).bg(bg),
                    ));
                }

                spans.push(Span::styled("  7d ", label_style.clone()));
                spans.push(Span::styled(
                    format!("{}%", seven_pct),
                    Style::default().fg(seven_color).bg(bg),
                ));
                if let Some(reset) = usage
                    .seven_day_resets_at
                    .as_deref()
                    .map(crate::usage::format_reset_time)
                {
                    spans.push(Span::styled(
                        format!(" → {}", reset),
                        Style::default().fg(dim).bg(bg),
                    ));
                }
            }
        } else if matches!(usage.provider, UsageProvider::CostBased) && usage.total_cost > 0.0 {
            spans.push(sep.clone());
            spans.push(Span::styled("cost ", label_style));
            spans.push(Span::styled(
                format!("${:.2}", usage.total_cost),
                value_style,
            ));
            if let Some(delta) = app.last_turn_cost_delta() {
                spans.push(Span::styled(
                    format!(" (+${:.2})", delta),
                    Style::default().fg(accent).bg(bg),
                ));
            }
        }
    }

    // Pad remainder with bg color so the strip fills the width
    let used_width: usize = spans
        .iter()
        .map(|s| s.content.chars().count())
        .sum();
    if used_width < width {
        spans.push(Span::styled(
            " ".repeat(width - used_width),
            Style::default().bg(bg),
        ));
    }

    Line::from(spans)
}

const FIVE_HOUR_CRIT_PCT: u32 = 90;
const FIVE_HOUR_WARN_PCT: u32 = 80;

/// Bands for 7-day usage (Claude Code style: green → blue → yellow → warn).
/// Each entry is `(minimum percent, color)`; first matching entry wins.
const SEVEN_DAY_BANDS: [(u32, Color); 4] = [
    (FIVE_HOUR_WARN_PCT, Color::Rgb(220, 160, 60)),
    (75, Color::Rgb(220, 200, 80)),
    (50, Color::Rgb(100, 160, 220)),
    (25, Color::Rgb(120, 200, 120)),
];

fn five_hour_color(pct: u32, warn: Color, tick: u64) -> Color {
    if pct >= FIVE_HOUR_CRIT_PCT {
        // Blink at ~1 Hz: tick is the workspace-animation tick which advances
        // ~once per render frame. Flipping on the parity of `tick / 30` gives
        // roughly 0.5s on / 0.5s off at 60 fps and stays stable when idle.
        if (tick / 30) % 2 == 0 {
            warn
        } else {
            Color::Rgb(140, 80, 30)
        }
    } else if pct >= FIVE_HOUR_WARN_PCT {
        warn
    } else {
        Color::Rgb(190, 190, 190)
    }
}

fn seven_day_band_color(pct: u32, warn: Color) -> Color {
    if pct >= FIVE_HOUR_CRIT_PCT {
        return warn;
    }
    for (min_pct, color) in SEVEN_DAY_BANDS.iter() {
        if pct >= *min_pct {
            return *color;
        }
    }
    Color::Rgb(190, 190, 190)
}

fn shorten_model(model: &str) -> String {
    let model = model
        .trim_start_matches("claude-")
        .trim_start_matches("gpt-")
        .trim_start_matches("gemini-");
    if model.len() > 28 {
        format!("{}…", &model[..27])
    } else {
        model.to_string()
    }
}

fn build_auth_dots(auth: &AuthStatus, bg: Color) -> Vec<Span<'static>> {
    fn dot(state: AuthState) -> (&'static str, Color) {
        match state {
            AuthState::Available => ("●", Color::Rgb(100, 200, 100)),
            AuthState::Expired => ("◐", Color::Rgb(255, 200, 100)),
            AuthState::NotConfigured => ("○", Color::Rgb(70, 70, 70)),
        }
    }

    let providers: &[(&str, AuthState)] = &[
        ("anthropic", auth.anthropic.state),
        ("openai", auth.openai),
        ("openrouter", auth.openrouter),
    ];

    let configured: Vec<_> = providers
        .iter()
        .filter(|(_, state)| !matches!(state, AuthState::NotConfigured))
        .collect();

    if configured.is_empty() {
        return Vec::new();
    }

    let mut spans = Vec::new();
    for (i, (name, state)) in configured.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" ", Style::default().bg(bg)));
        }
        let (ch, color) = dot(*state);
        spans.push(Span::styled(
            ch.to_string(),
            Style::default().fg(color).bg(bg),
        ));
        spans.push(Span::styled(
            name.to_string(),
            Style::default().fg(Color::Rgb(120, 120, 120)).bg(bg),
        ));
    }
    spans
}
