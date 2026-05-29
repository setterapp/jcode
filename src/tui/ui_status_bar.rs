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
    let warn = Color::Rgb(220, 160, 60);
    // Palette matched to the target statusline mock (pink model, mint
    // context bar, yellow cost, cyan 5h, dim grey 7d).
    let pink = Color::Rgb(238, 153, 200);
    let mint_filled = Color::Rgb(120, 232, 180);
    let mint_text = Color::Rgb(140, 220, 170);
    let bar_empty = Color::Rgb(60, 95, 80);
    let yellow = Color::Rgb(230, 220, 90);
    let cyan = Color::Rgb(120, 175, 230);
    let grey = Color::Rgb(140, 140, 140);
    let accent = Color::Rgb(130, 180, 130);

    let sep_style = Style::default().fg(dim).bg(bg);
    let cost_style = Style::default().fg(yellow).bg(bg);
    let model_style = Style::default().fg(pink).bg(bg);
    let profile_style = Style::default().fg(pink).bg(bg);

    let sep = Span::styled("  │  ", sep_style);

    let mut spans: Vec<Span<'static>> = Vec::new();

    // Rate limit warning (leftmost when active).
    if let Some(remaining) = app.rate_limit_remaining() {
        let secs = remaining.as_secs();
        spans.push(Span::styled("⚠ rate:", Style::default().fg(warn).bg(bg)));
        spans.push(Span::styled(
            format!("{}s", secs),
            Style::default().fg(warn).bg(bg),
        ));
        spans.push(sep.clone());
    }

    // Profile (e.g. `dev`) — left of the model, no brackets, pink.
    if let Some(profile) = app.active_profile() {
        spans.push(Span::styled(profile, profile_style));
        spans.push(sep.clone());
    }

    // Humanized model label, including the `(1M context)` / `(200k context)`
    // suffix that adapts per selected model.
    let model_raw = app.provider_model();
    let model_stripped =
        jcode_client_core::strip_provider_prefix(&model_raw).to_string();
    let model_display = if model_stripped.is_empty() || model_stripped == "unknown" {
        "no model".to_string()
    } else {
        crate::tui::status_line_runner::humanize_model_id(&model_stripped)
    };
    spans.push(Span::styled(model_display, model_style));

    // Context window — bar + percentage, sized from the active model.
    // Render even before the first turn so the bar is visible at idle.
    if let Some(limit) = app.context_limit() {
        if limit > 0 {
            let used = app.context_used_percent().clamp(0.0, 100.0);
            let ratio = used / 100.0;
            let pct = used.round() as u32;
            let bar_width = 16usize;
            let filled = ((ratio * bar_width as f64).round() as usize).min(bar_width);
            let empty = bar_width - filled;
            let (bar_fg, pct_fg) = if pct >= 80 {
                (warn, warn)
            } else {
                (mint_filled, mint_text)
            };
            spans.push(sep.clone());
            if filled > 0 {
                spans.push(Span::styled(
                    "█".repeat(filled),
                    Style::default().fg(bar_fg).bg(bg),
                ));
            }
            if empty > 0 {
                spans.push(Span::styled(
                    "█".repeat(empty),
                    Style::default().fg(bar_empty).bg(bg),
                ));
            }
            spans.push(Span::styled(
                format!(" {}%", pct),
                Style::default().fg(pct_fg).bg(bg),
            ));
        }
    }

    // Session cost (always when > 0, regardless of usage provider kind).
    let session_cost = widget_data
        .usage_info
        .as_ref()
        .map(|u| u.total_cost)
        .unwrap_or(0.0);
    if session_cost > 0.0 {
        spans.push(sep.clone());
        spans.push(Span::styled(format!("${:.2}", session_cost), cost_style));
        if let Some(delta) = app.last_turn_cost_delta() {
            spans.push(Span::styled(
                format!(" (+${:.2})", delta),
                Style::default().fg(accent).bg(bg),
            ));
        }
    }

    // 5h / 7d rate-limit windows with time-to-reset.
    if let Some(usage) = widget_data.usage_info.as_ref() {
        let is_rate_limit_provider = matches!(
            usage.provider,
            UsageProvider::Anthropic | UsageProvider::OpenAI | UsageProvider::Copilot
        );

        if is_rate_limit_provider {
            let five_pct = (usage.five_hour * 100.0) as u32;
            let seven_pct = (usage.seven_day * 100.0) as u32;

            if five_pct > 0 || seven_pct > 0 || usage.available {
                let tick = app.workspace_animation_tick();
                let five_color = if five_pct >= FIVE_HOUR_WARN_PCT {
                    five_hour_color(five_pct, warn, tick)
                } else {
                    cyan
                };
                let seven_color = if seven_pct >= FIVE_HOUR_WARN_PCT {
                    seven_day_band_color(seven_pct, warn)
                } else {
                    grey
                };

                spans.push(sep.clone());
                spans.push(Span::styled(
                    format!("5h {}%", five_pct),
                    Style::default().fg(five_color).bg(bg),
                ));
                if let Some(reset) = usage
                    .five_hour_resets_at
                    .as_deref()
                    .map(crate::usage::format_reset_time)
                {
                    spans.push(Span::styled(
                        format!(" → {}", reset),
                        Style::default().fg(five_color).bg(bg),
                    ));
                }

                spans.push(sep.clone());
                spans.push(Span::styled(
                    format!("7d {}%", seven_pct),
                    Style::default().fg(seven_color).bg(bg),
                ));
                if let Some(reset) = usage
                    .seven_day_resets_at
                    .as_deref()
                    .map(crate::usage::format_reset_time)
                {
                    spans.push(Span::styled(
                        format!(" → {}", reset),
                        Style::default().fg(seven_color).bg(bg),
                    ));
                }
            }
        }
    }

    // Pad remainder with bg color so the strip fills the width. Uses display
    // columns (not scalar count) so emoji / CJK names don't leave a raw gap
    // at the right edge.
    let used_width: usize = spans
        .iter()
        .map(|s| unicode_width::UnicodeWidthStr::width(s.content.as_ref()))
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

