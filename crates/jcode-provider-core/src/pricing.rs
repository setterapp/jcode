use crate::{RouteCheapnessEstimate, RouteCostConfidence, RouteCostSource};

fn usd_to_micros(usd: f64) -> u64 {
    (usd * 1_000_000.0).round() as u64
}

fn usd_per_token_str_to_micros_per_mtok(raw: &str) -> Option<u64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .map(|usd_per_token| (usd_per_token * 1_000_000_000_000.0).round() as u64)
}

pub fn anthropic_api_pricing(model: &str) -> Option<RouteCheapnessEstimate> {
    let base = model.strip_suffix("[1m]").unwrap_or(model);
    let long_context = model.ends_with("[1m]");
    match base {
        // Opus 4.6/4.7/4.8 share the $5/$25 metered rate (cache read $0.50,
        // cache write 1.25x input). Long-context [1m] doubles input/output.
        "claude-opus-4-8" | "claude-opus-4-7" | "claude-opus-4-6" => Some(
            RouteCheapnessEstimate::metered(
                RouteCostSource::PublicApiPricing,
                RouteCostConfidence::Exact,
                usd_to_micros(if long_context { 10.0 } else { 5.0 }),
                usd_to_micros(if long_context { 37.5 } else { 25.0 }),
                Some(usd_to_micros(if long_context { 1.0 } else { 0.5 })),
                Some(if long_context {
                    "Anthropic API long-context pricing".to_string()
                } else {
                    "Anthropic API pricing".to_string()
                }),
            )
            .with_cache_write(usd_to_micros(if long_context { 12.5 } else { 6.25 })),
        ),
        "claude-sonnet-4-6" => Some(
            RouteCheapnessEstimate::metered(
                RouteCostSource::PublicApiPricing,
                RouteCostConfidence::Exact,
                usd_to_micros(if long_context { 6.0 } else { 3.0 }),
                usd_to_micros(if long_context { 22.5 } else { 15.0 }),
                Some(usd_to_micros(if long_context { 0.6 } else { 0.3 })),
                Some(if long_context {
                    "Anthropic API long-context pricing".to_string()
                } else {
                    "Anthropic API pricing".to_string()
                }),
            )
            .with_cache_write(usd_to_micros(if long_context { 7.5 } else { 3.75 })),
        ),
        "claude-haiku-4-5" => Some(
            RouteCheapnessEstimate::metered(
                RouteCostSource::PublicApiPricing,
                RouteCostConfidence::Exact,
                usd_to_micros(1.0),
                usd_to_micros(5.0),
                Some(usd_to_micros(0.1)),
                Some("Anthropic API pricing".to_string()),
            )
            .with_cache_write(usd_to_micros(1.25)),
        ),
        "claude-opus-4-5" => Some(
            RouteCheapnessEstimate::metered(
                RouteCostSource::Heuristic,
                RouteCostConfidence::Medium,
                usd_to_micros(5.0),
                usd_to_micros(25.0),
                Some(usd_to_micros(0.5)),
                Some("Estimated from Opus 4.6 API pricing".to_string()),
            )
            .with_cache_write(usd_to_micros(6.25)),
        ),
        "claude-sonnet-4-5" | "claude-sonnet-4-20250514" => Some(
            RouteCheapnessEstimate::metered(
                RouteCostSource::Heuristic,
                RouteCostConfidence::Medium,
                usd_to_micros(3.0),
                usd_to_micros(15.0),
                Some(usd_to_micros(0.3)),
                Some("Estimated from Sonnet 4.6 API pricing".to_string()),
            )
            .with_cache_write(usd_to_micros(3.75)),
        ),
        _ => None,
    }
}

pub fn anthropic_oauth_pricing(model: &str, subscription: Option<&str>) -> RouteCheapnessEstimate {
    let base = model.strip_suffix("[1m]").unwrap_or(model);
    let is_opus = base.contains("opus");
    let is_1m = model.ends_with("[1m]");

    match subscription
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("max") => RouteCheapnessEstimate::subscription(
            RouteCostSource::RuntimePlan,
            RouteCostConfidence::Medium,
            usd_to_micros(100.0),
            None,
            Some(if is_opus {
                "Claude Max plan; Opus access included; 1M context".to_string()
            } else {
                "Claude Max plan; 1M context".to_string()
            }),
        ),
        Some("pro") => RouteCheapnessEstimate::subscription(
            RouteCostSource::RuntimePlan,
            RouteCostConfidence::Medium,
            usd_to_micros(20.0),
            None,
            Some(if is_1m {
                "Claude Pro plan; 1M context requires extra usage".to_string()
            } else {
                "Claude Pro plan".to_string()
            }),
        ),
        Some(other) => RouteCheapnessEstimate::subscription(
            RouteCostSource::RuntimePlan,
            RouteCostConfidence::Low,
            usd_to_micros(20.0),
            None,
            Some(format!(
                "Claude OAuth plan '{}'; assumed Pro-like pricing",
                other
            )),
        ),
        None => RouteCheapnessEstimate::subscription(
            RouteCostSource::PublicPlanPricing,
            RouteCostConfidence::Low,
            usd_to_micros(if is_opus { 100.0 } else { 20.0 }),
            None,
            Some(if is_opus {
                "Opus access implies Claude Max-like subscription pricing".to_string()
            } else {
                "Claude OAuth subscription pricing (plan not detected)".to_string()
            }),
        ),
    }
}

pub fn openai_api_pricing(model: &str) -> Option<RouteCheapnessEstimate> {
    let base = model.strip_suffix("[1m]").unwrap_or(model);
    match base {
        "gpt-5.5" => Some(RouteCheapnessEstimate::metered(
            RouteCostSource::PublicApiPricing,
            RouteCostConfidence::High,
            usd_to_micros(5.0),
            usd_to_micros(30.0),
            Some(usd_to_micros(0.5)),
            Some("OpenAI API pricing".to_string()),
        )),
        "gpt-5.4" | "gpt-5.4-pro" => Some(RouteCheapnessEstimate::metered(
            RouteCostSource::PublicApiPricing,
            RouteCostConfidence::High,
            usd_to_micros(2.5),
            usd_to_micros(15.0),
            Some(usd_to_micros(0.25)),
            Some("OpenAI API pricing".to_string()),
        )),
        "gpt-5.3-codex" | "gpt-5.2-codex" | "gpt-5.2" | "gpt-5.1" | "gpt-5.1-codex" => {
            Some(RouteCheapnessEstimate::metered(
                RouteCostSource::Heuristic,
                RouteCostConfidence::Low,
                usd_to_micros(2.5),
                usd_to_micros(15.0),
                Some(usd_to_micros(0.25)),
                Some("Estimated from GPT-5.4 API pricing".to_string()),
            ))
        }
        "gpt-5.3-codex-spark" | "gpt-5.1-codex-mini" => Some(RouteCheapnessEstimate::metered(
            RouteCostSource::Heuristic,
            RouteCostConfidence::Low,
            usd_to_micros(0.25),
            usd_to_micros(2.0),
            Some(usd_to_micros(0.025)),
            Some("Estimated from GPT-5 mini API pricing".to_string()),
        )),
        "gpt-5.1-codex-max"
        | "gpt-5.2-pro"
        | "gpt-5-chat-latest"
        | "gpt-5.1-chat-latest"
        | "gpt-5.2-chat-latest"
        | "gpt-5-codex"
        | "gpt-5" => Some(RouteCheapnessEstimate::metered(
            RouteCostSource::Heuristic,
            RouteCostConfidence::Low,
            usd_to_micros(2.5),
            usd_to_micros(15.0),
            Some(usd_to_micros(0.25)),
            Some("Estimated from GPT-5.4 API pricing".to_string()),
        )),
        _ => None,
    }
}

pub fn openai_oauth_pricing(model: &str) -> RouteCheapnessEstimate {
    let base = model.strip_suffix("[1m]").unwrap_or(model);
    let likely_pro = base.contains("pro") || matches!(base, "gpt-5.5" | "gpt-5.4");
    RouteCheapnessEstimate::subscription(
        RouteCostSource::PublicPlanPricing,
        RouteCostConfidence::Low,
        usd_to_micros(if likely_pro { 200.0 } else { 20.0 }),
        None,
        Some(if likely_pro {
            "ChatGPT subscription estimate; advanced GPT-5 access treated as Pro-like".to_string()
        } else {
            "ChatGPT subscription estimate".to_string()
        }),
    )
}

pub fn copilot_pricing(model: &str, zero_premium_mode: bool) -> RouteCheapnessEstimate {
    let likely_premium_model =
        model.contains("opus") || model.contains("gpt-5.5") || model.contains("gpt-5.4");
    let monthly_price = if likely_premium_model {
        usd_to_micros(39.0)
    } else {
        usd_to_micros(10.0)
    };
    let included_requests = if likely_premium_model { 1_500 } else { 300 };
    let estimated_reference = if zero_premium_mode {
        Some(0)
    } else {
        Some(monthly_price / included_requests)
    };

    RouteCheapnessEstimate::included_quota(
        RouteCostSource::RuntimePlan,
        if zero_premium_mode {
            RouteCostConfidence::High
        } else {
            RouteCostConfidence::Medium
        },
        monthly_price,
        Some(included_requests),
        estimated_reference,
        Some(if zero_premium_mode {
            "Copilot zero-premium mode: jcode will send requests as agent/non-premium when possible"
                .to_string()
        } else if likely_premium_model {
            "Copilot premium-request estimate using Pro+/premium pricing".to_string()
        } else {
            "Copilot estimate using Pro included premium requests".to_string()
        }),
    )
}

/// Hardcoded public API pricing for direct OpenAI-compatible providers
/// (DeepSeek, Z.AI/GLM, Moonshot/Kimi) whose own `/models` endpoint does not
/// return token prices, so the OpenRouter catalog path yields nothing. Prices
/// in USD per Mtok; `input` is the cache-miss rate and `cache_read` is the
/// provider's cache-hit input rate.
pub fn direct_compatible_pricing(model: &str) -> Option<RouteCheapnessEstimate> {
    let m = model.trim().to_ascii_lowercase();
    let metered = |input: f64, output: f64, cache_read: Option<f64>, note: &str| {
        Some(RouteCheapnessEstimate::metered(
            RouteCostSource::PublicApiPricing,
            RouteCostConfidence::High,
            usd_to_micros(input),
            usd_to_micros(output),
            cache_read.map(usd_to_micros),
            Some(note.to_string()),
        ))
    };

    if m.contains("deepseek") {
        if m.contains("v4-flash") || m.contains("v4flash") {
            return metered(0.14, 0.28, Some(0.0028), "DeepSeek V4 Flash API pricing");
        }
        if m.contains("v4") {
            return metered(0.435, 0.87, Some(0.003625), "DeepSeek V4 Pro API pricing");
        }
        return metered(0.28, 0.42, Some(0.028), "DeepSeek API pricing (estimated)");
    }

    if m.contains("glm") || m.contains("zai") || m.contains("z-ai") {
        if m.contains("4.7") || m.contains("4-7") || m.contains("5.1") || m.contains("51") {
            return metered(0.40, 1.75, None, "Z.AI GLM-4.7 API pricing");
        }
        if m.contains("4.6") || m.contains("4-6") {
            return metered(0.50, 2.00, None, "Z.AI GLM-4.6 API pricing (estimated)");
        }
        return metered(0.60, 2.20, None, "Z.AI GLM-4.5 API pricing");
    }

    if m.contains("kimi") || m.contains("moonshot") || m.contains("k2") {
        if m.contains("k2.6") || m.contains("k2-6") {
            return metered(0.95, 4.0, Some(0.16), "Moonshot Kimi K2.6 API pricing");
        }
        return metered(0.60, 3.0, Some(0.10), "Moonshot Kimi K2.5 API pricing");
    }

    None
}

pub fn openrouter_pricing_from_token_prices(
    prompt: Option<&str>,
    completion: Option<&str>,
    input_cache_read: Option<&str>,
    source: RouteCostSource,
    confidence: RouteCostConfidence,
    note: Option<String>,
) -> Option<RouteCheapnessEstimate> {
    let input = prompt.and_then(usd_per_token_str_to_micros_per_mtok)?;
    let output = completion.and_then(usd_per_token_str_to_micros_per_mtok)?;
    let cache = input_cache_read.and_then(usd_per_token_str_to_micros_per_mtok);
    Some(RouteCheapnessEstimate::metered(
        source, confidence, input, output, cache, note,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RouteBillingKind;

    #[test]
    fn anthropic_api_pricing_handles_long_context_variants() {
        let estimate = anthropic_api_pricing("claude-opus-4-6[1m]").expect("priced model");
        assert_eq!(estimate.billing_kind, RouteBillingKind::Metered);
        assert_eq!(estimate.source, RouteCostSource::PublicApiPricing);
        assert_eq!(estimate.confidence, RouteCostConfidence::Exact);
        assert_eq!(estimate.input_price_per_mtok_micros, Some(10_000_000));
        assert_eq!(estimate.output_price_per_mtok_micros, Some(37_500_000));
        assert_eq!(estimate.cache_read_price_per_mtok_micros, Some(1_000_000));
    }

    #[test]
    fn openrouter_token_pricing_parses_token_prices() {
        let estimate = openrouter_pricing_from_token_prices(
            Some("0.0000025"),
            Some("0.000015"),
            Some("0.00000025"),
            RouteCostSource::OpenRouterCatalog,
            RouteCostConfidence::Medium,
            Some("test".to_string()),
        )
        .expect("parsed pricing");

        assert_eq!(estimate.input_price_per_mtok_micros, Some(2_500_000));
        assert_eq!(estimate.output_price_per_mtok_micros, Some(15_000_000));
        assert_eq!(estimate.cache_read_price_per_mtok_micros, Some(250_000));
    }

    #[test]
    fn copilot_zero_mode_marks_estimate_high_confidence_and_zero_reference_cost() {
        let estimate = copilot_pricing("claude-opus-4-6", true);
        assert_eq!(estimate.billing_kind, RouteBillingKind::IncludedQuota);
        assert_eq!(estimate.confidence, RouteCostConfidence::High);
        assert_eq!(estimate.estimated_reference_cost_micros, Some(0));
    }
}
