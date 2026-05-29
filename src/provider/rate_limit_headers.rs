//! Parse provider rate-limit response headers into a normalized update.
//!
//! Live rate-limit data extracted from every streaming response — keeps the
//! statusline 5h/7d numbers fresh between the 5-minute `/api/oauth/usage` poll
//! cycles. Header names confirmed against Claude Code (`anthropic-ratelimit-unified-*`)
//! and provider docs.

use reqwest::header::HeaderMap;
use std::time::Instant;

/// Which provider produced this update — drives which usage cache slot it writes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitProvider {
    Anthropic,
    OpenAI,
    OpenRouter,
    Copilot,
}

/// A snapshot of rate-limit state extracted from response headers.
///
/// Percentages are 0.0–100.0 (matching Anthropic's `utilization` integer field).
/// `resets_at_iso` is an ISO-8601 string when available (Anthropic returns epoch
/// seconds — we convert to ISO so it matches the existing `UsageData` schema).
#[derive(Debug, Clone)]
pub struct RateLimitUpdate {
    pub provider: RateLimitProvider,
    pub five_hour_used_pct: Option<f64>,
    pub seven_day_used_pct: Option<f64>,
    pub five_hour_resets_at_iso: Option<String>,
    pub seven_day_resets_at_iso: Option<String>,
    pub tokens_remaining: Option<u64>,
    pub captured_at: Instant,
}

impl RateLimitUpdate {
    pub fn new(provider: RateLimitProvider) -> Self {
        Self {
            provider,
            five_hour_used_pct: None,
            seven_day_used_pct: None,
            five_hour_resets_at_iso: None,
            seven_day_resets_at_iso: None,
            tokens_remaining: None,
            captured_at: Instant::now(),
        }
    }

    pub fn has_data(&self) -> bool {
        self.five_hour_used_pct.is_some()
            || self.seven_day_used_pct.is_some()
            || self.tokens_remaining.is_some()
    }

    /// Parse Anthropic's unified rate-limit headers.
    ///
    /// Header names (from Claude Code source `services/mockRateLimits.ts`):
    /// - `anthropic-ratelimit-unified-5h-utilization` — integer percent 0..100
    /// - `anthropic-ratelimit-unified-5h-reset` — epoch seconds
    /// - `anthropic-ratelimit-unified-7d-utilization`
    /// - `anthropic-ratelimit-unified-7d-reset`
    pub fn from_anthropic_headers(headers: &HeaderMap) -> Option<Self> {
        let mut update = Self::new(RateLimitProvider::Anthropic);

        update.five_hour_used_pct =
            header_str(headers, "anthropic-ratelimit-unified-5h-utilization")
                .and_then(|s| s.trim().parse::<f64>().ok());
        update.five_hour_resets_at_iso =
            header_str(headers, "anthropic-ratelimit-unified-5h-reset").and_then(epoch_to_iso);

        update.seven_day_used_pct =
            header_str(headers, "anthropic-ratelimit-unified-7d-utilization")
                .and_then(|s| s.trim().parse::<f64>().ok());
        update.seven_day_resets_at_iso =
            header_str(headers, "anthropic-ratelimit-unified-7d-reset").and_then(epoch_to_iso);

        update.tokens_remaining =
            header_str(headers, "anthropic-ratelimit-tokens-remaining")
                .and_then(|s| s.trim().parse::<u64>().ok());

        if update.has_data() { Some(update) } else { None }
    }

    /// Parse OpenAI-style rate-limit headers.
    ///
    /// OpenAI doesn't expose a 5h/7d window — only per-minute token/request
    /// budgets. We surface `x-ratelimit-remaining-tokens` for now; the 5h/7d
    /// slots are left None so the existing OpenAI usage poll keeps owning them.
    pub fn from_openai_headers(headers: &HeaderMap) -> Option<Self> {
        let mut update = Self::new(RateLimitProvider::OpenAI);
        update.tokens_remaining = header_str(headers, "x-ratelimit-remaining-tokens")
            .and_then(|s| s.trim().parse::<u64>().ok());
        if update.has_data() { Some(update) } else { None }
    }

    /// Parse OpenRouter rate-limit headers (mirrors OpenAI's pattern).
    pub fn from_openrouter_headers(headers: &HeaderMap) -> Option<Self> {
        let mut update = Self::new(RateLimitProvider::OpenRouter);
        update.tokens_remaining = header_str(headers, "x-ratelimit-remaining")
            .and_then(|s| s.trim().parse::<u64>().ok());
        if update.has_data() { Some(update) } else { None }
    }

    /// Parse GitHub Copilot rate-limit headers (`x-ratelimit-*`).
    pub fn from_copilot_headers(headers: &HeaderMap) -> Option<Self> {
        let mut update = Self::new(RateLimitProvider::Copilot);
        update.tokens_remaining = header_str(headers, "x-ratelimit-remaining")
            .and_then(|s| s.trim().parse::<u64>().ok());
        if update.has_data() { Some(update) } else { None }
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

fn epoch_to_iso(value: &str) -> Option<String> {
    let secs: i64 = value.trim().parse().ok()?;
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0).map(|dt| dt.to_rfc3339())
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue};

    fn hm(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (k, v) in pairs {
            headers.insert(*k, HeaderValue::from_static(*v));
        }
        headers
    }

    #[test]
    fn anthropic_full_payload_parses() {
        let h = hm(&[
            ("anthropic-ratelimit-unified-5h-utilization", "42"),
            ("anthropic-ratelimit-unified-5h-reset", "1747500000"),
            ("anthropic-ratelimit-unified-7d-utilization", "13"),
            ("anthropic-ratelimit-unified-7d-reset", "1748000000"),
        ]);
        let update = RateLimitUpdate::from_anthropic_headers(&h).expect("update");
        assert_eq!(update.provider, RateLimitProvider::Anthropic);
        assert_eq!(update.five_hour_used_pct, Some(42.0));
        assert_eq!(update.seven_day_used_pct, Some(13.0));
        assert!(update.five_hour_resets_at_iso.as_deref().unwrap().starts_with("2025-"));
    }

    #[test]
    fn anthropic_returns_none_without_any_unified_headers() {
        let h = hm(&[("content-type", "application/json")]);
        assert!(RateLimitUpdate::from_anthropic_headers(&h).is_none());
    }

    #[test]
    fn anthropic_partial_5h_only_still_parses() {
        let h = hm(&[("anthropic-ratelimit-unified-5h-utilization", "77")]);
        let update = RateLimitUpdate::from_anthropic_headers(&h).expect("update");
        assert_eq!(update.five_hour_used_pct, Some(77.0));
        assert_eq!(update.seven_day_used_pct, None);
    }

    #[test]
    fn openai_remaining_tokens_parses() {
        let h = hm(&[("x-ratelimit-remaining-tokens", "12000")]);
        let update = RateLimitUpdate::from_openai_headers(&h).expect("update");
        assert_eq!(update.tokens_remaining, Some(12000));
    }
}
