use super::*;

/// Update cost calculation based on token usage (for API-key providers)
impl App {
    pub(super) fn current_streaming_tps_elapsed(&self) -> Duration {
        let mut elapsed = self.streaming_tps_elapsed;
        if let Some(start) = self.streaming_tps_start {
            elapsed += start.elapsed();
        }
        elapsed
    }

    pub(super) fn snapshot_streaming_tps(&mut self) {
        self.streaming_tps_observed_output_tokens = self.streaming_total_output_tokens;
        self.streaming_tps_observed_elapsed = self.current_streaming_tps_elapsed();
    }

    pub(super) fn resume_streaming_tps(&mut self) {
        self.streaming_tps_collect_output = true;
        if self.streaming_tps_start.is_none() {
            self.streaming_tps_start = Some(Instant::now());
        }
    }

    pub(super) fn pause_streaming_tps(&mut self, keep_collecting_output: bool) {
        if let Some(start) = self.streaming_tps_start.take() {
            self.streaming_tps_elapsed += start.elapsed();
        }
        self.streaming_tps_collect_output = keep_collecting_output;
    }

    pub(super) fn reset_streaming_tps(&mut self) {
        self.streaming_tps_start = None;
        self.streaming_tps_elapsed = Duration::ZERO;
        self.streaming_tps_collect_output = false;
        self.streaming_total_output_tokens = 0;
        self.streaming_tps_observed_output_tokens = 0;
        self.streaming_tps_observed_elapsed = Duration::ZERO;
    }

    pub(super) fn open_usage_inline_loading(&mut self) {
        self.push_usage_loading_card();
        self.inline_interactive_state = None;
        self.inline_view_state = None;
        self.input.clear();
        self.cursor_pos = 0;
        self.set_status_notice("Usage → refreshing");
    }

    /// Request a background usage refresh that only updates the inline strip —
    /// no chat card is inserted. Used for automatic fetches (startup, periodic).
    pub(super) fn request_usage_report_silent(&mut self) {
        self.usage_report_silent = true;
        self.request_usage_report();
    }

    pub(super) fn request_usage_report(&mut self) {
        use crate::bus::{Bus, BusEvent};

        if self.usage_report_refreshing {
            return;
        }
        self.usage_report_refreshing = true;

        let publish = || async move {
            let results = crate::usage::fetch_all_provider_usage_progressive(|progress| {
                Bus::global().publish(BusEvent::UsageReportProgress(progress));
            })
            .await;
            Bus::global().publish(BusEvent::UsageReport(results));
        };

        if tokio::runtime::Handle::try_current().is_ok() {
            tokio::spawn(publish());
        } else {
            std::thread::spawn(move || {
                if let Ok(runtime) = tokio::runtime::Runtime::new() {
                    runtime.block_on(publish());
                }
            });
        }
    }

    pub(super) fn update_cost_impl(&mut self) {
        let provider_name = self.provider.name().to_lowercase();

        // Only metered API-key providers have a per-token cost to show.
        if !provider_name.contains("openrouter")
            && !provider_name.contains("anthropic")
            && !provider_name.contains("claude")
            && !provider_name.contains("openai")
        {
            return;
        }

        // OAuth (subscription) cost is amortized in the plan, not metered.
        let is_oauth = (provider_name.contains("anthropic") || provider_name.contains("claude"))
            && std::env::var("ANTHROPIC_API_KEY").is_err();
        if is_oauth {
            return;
        }

        // Resolve real per-model pricing (micros per Mtok) for the active route.
        let model = self.provider.model();
        let api_method = if provider_name.contains("openrouter") {
            "openrouter"
        } else if provider_name.contains("openai") {
            "openai-api-key"
        } else {
            "api-key"
        };
        let estimate =
            crate::provider::pricing::cheapness_for_route(&model, self.provider.name(), api_method);
        let field = |v: Option<u64>, fallback: f64| -> f64 {
            v.map(|m| m as f64).unwrap_or(fallback)
        };

        // micros per Mtok; fall back to the legacy flat defaults when unknown.
        let input_micros = field(
            estimate.as_ref().and_then(|e| e.input_price_per_mtok_micros),
            15_000_000.0,
        );
        let output_micros = field(
            estimate
                .as_ref()
                .and_then(|e| e.output_price_per_mtok_micros),
            60_000_000.0,
        );
        // Cache read/write default to full input price (providers without a cache
        // discount/premium bill cache tokens as normal input).
        let cache_read_micros = field(
            estimate
                .as_ref()
                .and_then(|e| e.cache_read_price_per_mtok_micros),
            input_micros,
        );
        let cache_write_micros = field(
            estimate
                .as_ref()
                .and_then(|e| e.cache_write_price_per_mtok_micros),
            input_micros,
        );

        // Surface the resolved $/Mtok in the debug panel.
        self.cached_prompt_price = Some((input_micros / 1_000_000.0) as f32);
        self.cached_completion_price = Some((output_micros / 1_000_000.0) as f32);

        let cache_read_tokens = self.streaming_cache_read_tokens.unwrap_or(0);
        let cache_write_tokens = self.streaming_cache_creation_tokens.unwrap_or(0);
        // Anthropic reports input excluding cache; OpenAI/DeepSeek fold cached
        // tokens into prompt_tokens. Normalize to full-price input for both.
        let is_anthropic = provider_name.contains("anthropic") || provider_name.contains("claude");
        let full_input_tokens = if is_anthropic {
            self.streaming_input_tokens
        } else {
            self.streaming_input_tokens
                .saturating_sub(cache_read_tokens)
                .saturating_sub(cache_write_tokens)
        };

        // cost = tokens * (micros per Mtok) / 1e12 = dollars.
        let cost_usd = ((full_input_tokens as f64) * input_micros
            + (cache_read_tokens as f64) * cache_read_micros
            + (cache_write_tokens as f64) * cache_write_micros
            + (self.streaming_output_tokens as f64) * output_micros)
            / 1_000_000_000_000.0;

        let delta = cost_usd as f32;
        self.total_cost += delta;
        if delta > 0.0 {
            self.last_turn_cost_delta = delta;
            self.last_turn_cost_at = Some(Instant::now());
        }
    }

    pub(super) fn compute_streaming_tps(&self) -> Option<f32> {
        let elapsed_secs = self.streaming_tps_observed_elapsed.as_secs_f32();
        let total_tokens = self.streaming_tps_observed_output_tokens;
        if elapsed_secs > 0.1 && total_tokens > 0 {
            Some(total_tokens as f32 / elapsed_secs)
        } else {
            None
        }
    }

    pub(super) fn handle_changelog_key(&mut self, code: KeyCode) -> Result<()> {
        let scroll = self.changelog_scroll.unwrap_or(0);
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.changelog_scroll = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.changelog_scroll = Some(scroll.saturating_add(1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.changelog_scroll = Some(scroll.saturating_sub(1));
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                self.changelog_scroll = Some(scroll.saturating_add(20));
            }
            KeyCode::PageUp => {
                self.changelog_scroll = Some(scroll.saturating_sub(20));
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.changelog_scroll = Some(0);
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.changelog_scroll = Some(usize::MAX);
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn handle_help_key(&mut self, code: KeyCode) -> Result<()> {
        let scroll = self.help_scroll.unwrap_or(0);
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.help_scroll = None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.help_scroll = Some(scroll.saturating_add(1));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.help_scroll = Some(scroll.saturating_sub(1));
            }
            KeyCode::PageDown | KeyCode::Char(' ') => {
                self.help_scroll = Some(scroll.saturating_add(20));
            }
            KeyCode::PageUp => {
                self.help_scroll = Some(scroll.saturating_sub(20));
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.help_scroll = Some(0);
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.help_scroll = Some(usize::MAX);
            }
            _ => {}
        }
        Ok(())
    }
}
