use super::{InfoWidgetData, UsageInfo, UsageProvider};
use crate::tui::color_support::rgb;
use ratatui::prelude::*;
use unicode_width::UnicodeWidthStr;

pub(super) fn render_usage_widget(data: &InfoWidgetData, inner: Rect) -> Vec<Line<'static>> {
    let Some(info) = &data.usage_info else {
        return Vec::new();
    };
    if !info.available {
        return Vec::new();
    }

    match info.provider {
        UsageProvider::Copilot => {
            vec![Line::from(vec![Span::styled(
                format!(
                    "{} in + {} out",
                    format_tokens(info.input_tokens),
                    format_tokens(info.output_tokens)
                ),
                Style::default().fg(rgb(140, 140, 150)),
            )])]
        }
        UsageProvider::CostBased => {
            vec![
                Line::from(vec![
                    Span::styled("💰 ", Style::default().fg(rgb(140, 180, 255))),
                    Span::styled(
                        format!("${:.4}", info.total_cost),
                        Style::default().fg(rgb(180, 180, 190)).bold(),
                    ),
                ]),
                Line::from(vec![Span::styled(
                    format!(
                        "{} in + {} out",
                        format_tokens(info.input_tokens),
                        format_tokens(info.output_tokens)
                    ),
                    Style::default().fg(rgb(140, 140, 150)),
                )]),
            ]
        }
        _ => {
            let five_hr_used = (info.five_hour * 100.0).round().clamp(0.0, 100.0) as u8;
            let seven_day_used = (info.seven_day * 100.0).round().clamp(0.0, 100.0) as u8;
            let five_hr_left = 100u8.saturating_sub(five_hr_used);
            let seven_day_left = 100u8.saturating_sub(seven_day_used);

            let five_hr_reset = info
                .five_hour_resets_at
                .as_deref()
                .map(crate::usage::format_reset_time);
            let seven_day_reset = info
                .seven_day_resets_at
                .as_deref()
                .map(crate::usage::format_reset_time);

            let mut lines = Vec::new();
            let label = info.provider.label();
            if !label.is_empty() {
                lines.push(Line::from(vec![Span::styled(
                    format!("{} limits", label),
                    Style::default()
                        .fg(rgb(140, 140, 150))
                        .add_modifier(ratatui::style::Modifier::DIM),
                )]));
            }
            lines.push(render_labeled_bar(
                "5-hour",
                five_hr_used,
                five_hr_left,
                five_hr_reset.as_deref(),
                inner.width,
            ));
            lines.push(render_labeled_bar(
                "Weekly",
                seven_day_used,
                seven_day_left,
                seven_day_reset.as_deref(),
                inner.width,
            ));
            if let Some(spark_usage) = info.spark {
                let spark_used = (spark_usage * 100.0).round().clamp(0.0, 100.0) as u8;
                let spark_left = 100u8.saturating_sub(spark_used);
                let spark_reset = info
                    .spark_resets_at
                    .as_deref()
                    .map(crate::usage::format_reset_time);
                lines.push(render_labeled_bar(
                    "Spark",
                    spark_used,
                    spark_left,
                    spark_reset.as_deref(),
                    inner.width,
                ));
            }
            lines
        }
    }
}

pub(super) fn render_usage_compact(info: &UsageInfo, width: u16) -> Vec<Line<'static>> {
    if !info.available {
        return Vec::new();
    }

    let five_hr_used = (info.five_hour * 100.0).round().clamp(0.0, 100.0) as u8;
    let seven_day_used = (info.seven_day * 100.0).round().clamp(0.0, 100.0) as u8;
    let five_hr_left = 100u8.saturating_sub(five_hr_used);
    let seven_day_left = 100u8.saturating_sub(seven_day_used);
    let five_hr_reset = info
        .five_hour_resets_at
        .as_deref()
        .map(crate::usage::format_reset_time);
    let seven_day_reset = info
        .seven_day_resets_at
        .as_deref()
        .map(crate::usage::format_reset_time);

    let mut lines = Vec::new();
    let label = info.provider.label();
    if !label.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            format!("{} limits", label),
            Style::default()
                .fg(rgb(140, 140, 150))
                .add_modifier(ratatui::style::Modifier::DIM),
        )]));
    }
    lines.push(render_labeled_bar(
        "5-hour",
        five_hr_used,
        five_hr_left,
        five_hr_reset.as_deref(),
        width,
    ));
    lines.push(render_labeled_bar(
        "Weekly",
        seven_day_used,
        seven_day_left,
        seven_day_reset.as_deref(),
        width,
    ));
    if let Some(spark_usage) = info.spark {
        let spark_used = (spark_usage * 100.0).round().clamp(0.0, 100.0) as u8;
        let spark_left = 100u8.saturating_sub(spark_used);
        let spark_reset = info
            .spark_resets_at
            .as_deref()
            .map(crate::usage::format_reset_time);
        lines.push(render_labeled_bar(
            "Spark",
            spark_used,
            spark_left,
            spark_reset.as_deref(),
            width,
        ));
    }
    lines
}

fn render_labeled_bar(
    label: &str,
    used_pct: u8,
    left_pct: u8,
    reset_time: Option<&str>,
    width: u16,
) -> Line<'static> {
    let color = if left_pct <= 20 {
        rgb(255, 100, 100)
    } else if left_pct <= 50 {
        rgb(255, 200, 100)
    } else {
        rgb(100, 200, 100)
    };

    let label_width = 7;
    let suffix_width = 10;
    let bar_width = width
        .saturating_sub(label_width + 1 + suffix_width)
        .clamp(4, 12) as usize;

    let filled = ((used_pct as f32 / 100.0) * bar_width as f32).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    let bar_filled = "█".repeat(filled);
    let bar_empty = "░".repeat(empty);

    let suffix = if left_pct == 0 {
        if let Some(reset) = reset_time {
            format!(" resets {}", reset)
        } else {
            " 0% left".to_string()
        }
    } else {
        format!(" {}% left", left_pct)
    };

    let padded_label = format!("{:<7}", label);

    Line::from(vec![
        Span::styled(padded_label, Style::default().fg(rgb(140, 140, 150))),
        Span::styled(bar_filled, Style::default().fg(color)),
        Span::styled(bar_empty, Style::default().fg(rgb(50, 50, 60))),
        Span::styled(suffix, Style::default().fg(color)),
    ])
}

pub(super) fn render_usage_bar(
    used_tokens: usize,
    limit_tokens: usize,
    width: u16,
) -> Line<'static> {
    let safe_limit = limit_tokens.max(1);
    let bar_width = width.saturating_sub(2).min(24) as usize;
    if bar_width == 0 {
        return Line::default();
    }

    let mut used_cells = ((used_tokens as f64 / safe_limit as f64) * bar_width as f64)
        .round()
        .max(0.0) as usize;
    if used_cells > bar_width {
        used_cells = bar_width;
    }

    let used_pct = ((used_tokens as f64 / safe_limit as f64) * 100.0)
        .round()
        .clamp(0.0, 100.0) as u8;
    let left_pct = 100u8.saturating_sub(used_pct);
    let used_color = if left_pct <= 20 {
        rgb(255, 100, 100)
    } else if left_pct <= 50 {
        rgb(255, 200, 100)
    } else {
        rgb(100, 200, 100)
    };

    let label = format!(
        "{}/{}",
        format_token_k(used_tokens),
        format_token_k(limit_tokens)
    );
    let show_label = UnicodeWidthStr::width(label.as_str()) <= bar_width;
    let mut spans = Vec::new();
    spans.push(Span::styled("[", Style::default().fg(rgb(90, 90, 100))));
    if show_label {
        let label_start = (bar_width - label.len()) / 2;
        let label_end = label_start + label.len();
        for idx in 0..bar_width {
            let in_used = idx < used_cells;
            let base_char = if in_used { '█' } else { '░' };
            let ch = if idx >= label_start && idx < label_end {
                label.as_bytes()[idx - label_start] as char
            } else {
                base_char
            };
            let style = if idx >= label_start && idx < label_end {
                if in_used {
                    Style::default().fg(rgb(20, 30, 35)).bold()
                } else {
                    Style::default().fg(rgb(170, 170, 180)).bold()
                }
            } else if in_used {
                Style::default().fg(used_color)
            } else {
                Style::default().fg(rgb(50, 50, 60))
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
    } else {
        let empty_cells = bar_width.saturating_sub(used_cells);
        spans.push(Span::styled(
            "█".repeat(used_cells),
            Style::default().fg(used_color),
        ));
        if empty_cells > 0 {
            spans.push(Span::styled(
                "░".repeat(empty_cells),
                Style::default().fg(rgb(50, 50, 60)),
            ));
        }
    }
    spans.push(Span::styled("]", Style::default().fg(rgb(90, 90, 100))));
    Line::from(spans)
}

pub(super) fn render_context_usage_line(
    label: &str,
    used_tokens: usize,
    limit_tokens: usize,
    width: u16,
) -> Line<'static> {
    let label_width = UnicodeWidthStr::width(label);
    let bar_width = width.saturating_sub(label_width as u16 + 1);

    if bar_width < 3 {
        return Line::from(vec![
            Span::styled(label.to_string(), Style::default().fg(rgb(140, 140, 150))),
            Span::raw(" "),
            Span::styled(
                format!(
                    "{}/{}",
                    format_token_k(used_tokens),
                    format_token_k(limit_tokens)
                ),
                Style::default().fg(rgb(100, 200, 100)).bold(),
            ),
        ]);
    }

    let mut spans = vec![Span::styled(
        format!("{label} "),
        Style::default().fg(rgb(140, 140, 150)),
    )];
    spans.extend(render_usage_bar(used_tokens, limit_tokens, bar_width).spans);
    Line::from(spans)
}

fn format_token_k(tokens: usize) -> String {
    if tokens >= 1000 {
        format!("{}k", tokens / 1000)
    } else {
        format!("{}", tokens)
    }
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}

/// Render a single-line inline usage strip for display below the input area.
/// Shows 5h/Weekly (and Spark if present) all on one line, Claude Code style.
/// Returns an empty `Line` when usage data is not available.
pub fn render_usage_inline_strip(info: &UsageInfo, _width: u16) -> Line<'static> {
    // For CostBased/Copilot we only render when available.
    // For Anthropic/OpenAI we render always (shows 0% while data loads).
    let has_time_limits = matches!(
        info.provider,
        UsageProvider::Anthropic | UsageProvider::OpenAI
    );
    if !has_time_limits && !info.available {
        return Line::default();
    }

    match info.provider {
        UsageProvider::Copilot => {
            // Copilot shows token counts instead of time-windowed bars
            Line::from(vec![Span::styled(
                format!(
                    "  {} in + {} out",
                    format_tokens(info.input_tokens),
                    format_tokens(info.output_tokens)
                ),
                Style::default().fg(rgb(120, 120, 130)),
            )])
        }
        UsageProvider::CostBased => {
            Line::from(vec![
                Span::styled("  💰 ", Style::default().fg(rgb(140, 180, 255))),
                Span::styled(
                    format!("${:.4}", info.total_cost),
                    Style::default().fg(rgb(180, 180, 190)).bold(),
                ),
                Span::styled(
                    format!(
                        "  {} in + {} out",
                        format_tokens(info.input_tokens),
                        format_tokens(info.output_tokens)
                    ),
                    Style::default().fg(rgb(120, 120, 130)),
                ),
            ])
        }
        _ => {
            // Anthropic / OpenAI: show 5h + Weekly bars inline
            let five_hr_used = (info.five_hour * 100.0).round().clamp(0.0, 100.0) as u8;
            let seven_day_used = (info.seven_day * 100.0).round().clamp(0.0, 100.0) as u8;
            let five_hr_left = 100u8.saturating_sub(five_hr_used);
            let seven_day_left = 100u8.saturating_sub(seven_day_used);

            let five_hr_reset = info
                .five_hour_resets_at
                .as_deref()
                .map(crate::usage::format_reset_time);
            let seven_day_reset = info
                .seven_day_resets_at
                .as_deref()
                .map(crate::usage::format_reset_time);

            let mut spans: Vec<Span<'static>> = Vec::new();

            // Left padding
            spans.push(Span::raw("  "));

            // 5-hour bar segment
            append_inline_bar_spans(
                &mut spans,
                "5h",
                five_hr_used,
                five_hr_left,
                five_hr_reset.as_deref(),
            );

            // Separator
            spans.push(Span::styled(
                "   ",
                Style::default().fg(rgb(60, 60, 70)),
            ));

            // Weekly bar segment
            append_inline_bar_spans(
                &mut spans,
                "Weekly",
                seven_day_used,
                seven_day_left,
                seven_day_reset.as_deref(),
            );

            // Optional Spark bar
            if let Some(spark_usage) = info.spark {
                let spark_used = (spark_usage * 100.0).round().clamp(0.0, 100.0) as u8;
                let spark_left = 100u8.saturating_sub(spark_used);
                let spark_reset = info
                    .spark_resets_at
                    .as_deref()
                    .map(crate::usage::format_reset_time);
                spans.push(Span::styled("   ", Style::default().fg(rgb(60, 60, 70))));
                append_inline_bar_spans(
                    &mut spans,
                    "Spark",
                    spark_used,
                    spark_left,
                    spark_reset.as_deref(),
                );
            }

            Line::from(spans)
        }
    }
}

fn bar_color(left_pct: u8) -> Color {
    if left_pct <= 20 {
        rgb(255, 100, 100)
    } else if left_pct <= 50 {
        rgb(255, 200, 100)
    } else {
        rgb(100, 200, 100)
    }
}

/// Append spans for a single labeled mini-bar into `spans`.
fn append_inline_bar_spans(
    spans: &mut Vec<Span<'static>>,
    label: &str,
    used_pct: u8,
    left_pct: u8,
    reset_time: Option<&str>,
) {
    let color = bar_color(left_pct);
    let bar_width: usize = 8;
    let filled = ((used_pct as f32 / 100.0) * bar_width as f32).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    let suffix = if left_pct == 0 {
        if let Some(reset) = reset_time {
            format!(" resets {}", reset)
        } else {
            " 0% left".to_string()
        }
    } else {
        format!(" {}% left", left_pct)
    };

    spans.push(Span::styled(
        format!("{} ", label),
        Style::default().fg(rgb(120, 120, 130)),
    ));
    spans.push(Span::styled(
        "█".repeat(filled),
        Style::default().fg(color),
    ));
    spans.push(Span::styled(
        "░".repeat(empty),
        Style::default().fg(rgb(50, 50, 60)),
    ));
    spans.push(Span::styled(suffix, Style::default().fg(color)));
}
