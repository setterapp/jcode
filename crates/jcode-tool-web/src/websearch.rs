use anyhow::Result;
use serde_json::{json, Value};

use crate::types::{SearchEngine, WebSearchResult};

pub struct WebSearch;

impl WebSearch {
    pub async fn search(query: &str, max_results: usize) -> Result<Vec<WebSearchResult>> {
        let api_key = std::env::var("SERPAPI_API_KEY")
            .or_else(|_| std::env::var("EXA_API_KEY"))
            .or_else(|_| std::env::var("GOOGLE_API_KEY"));

        match api_key {
            Ok(_) if std::env::var("SERPAPI_API_KEY").is_ok() => {
                Self::serper_search(query, max_results).await
            }
            Ok(_) if std::env::var("EXA_API_KEY").is_ok() => {
                Self::exa_search(query, max_results).await
            }
            _ => Self::fallback_search(query, max_results).await,
        }
    }

    async fn exa_search(query: &str, max_results: usize) -> Result<Vec<WebSearchResult>> {
        let api_key = std::env::var("EXA_API_KEY")?;
        let client = reqwest::Client::new();
        let resp = client
            .post("https://api.exa.ai/search")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&json!({
                "query": query,
                "num_results": max_results
            }))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        let results = data["results"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|r| WebSearchResult {
                        title: r["title"].as_str().unwrap_or("").to_string(),
                        url: r["url"].as_str().unwrap_or("").to_string(),
                        snippet: r["snippet"].as_str().unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn serper_search(query: &str, max_results: usize) -> Result<Vec<WebSearchResult>> {
        let api_key = std::env::var("SERPAPI_API_KEY")?;
        let client = reqwest::Client::new();
        let resp = client
            .post("https://google.serper.dev/search")
            .header("X-API-KEY", &api_key)
            .header("Content-Type", "application/json")
            .json(&json!({
                "q": query,
                "num": max_results
            }))
            .send()
            .await?;

        let data: Value = resp.json().await?;
        let results = data["organic"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|r| WebSearchResult {
                        title: r["title"].as_str().unwrap_or("").to_string(),
                        url: r["link"].as_str().unwrap_or("").to_string(),
                        snippet: r["snippet"].as_str().unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    async fn fallback_search(_query: &str, _max_results: usize) -> Result<Vec<WebSearchResult>> {
        Err(anyhow::anyhow!(
            "No search API key configured. Set EXA_API_KEY or SERPAPI_API_KEY."
        ))
    }
}
