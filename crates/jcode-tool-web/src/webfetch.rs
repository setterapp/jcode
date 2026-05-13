use anyhow::Result;
use scraper::{Html, Selector};

use crate::types::WebFetchResult;

pub struct WebFetch;

impl WebFetch {
    pub async fn fetch(url: &str) -> Result<WebFetchResult> {
        let client = reqwest::Client::builder()
            .user_agent("jcode-plus/0.1")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let resp = client.get(url).send().await?;
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/plain")
            .to_string();
        let body = resp.text().await?;

        let content = if content_type.contains("text/html") {
            extract_text(&body)
        } else {
            body.clone()
        };

        Ok(WebFetchResult {
            url: url.to_string(),
            content,
            content_type,
            status,
        })
    }
}

fn extract_text(html: &str) -> String {
    let document = Html::parse_document(html);

    let selectors = [
        "article",
        "main",
        ".content",
        "#content",
        ".post-content",
        ".entry-content",
        "body",
    ];

    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element.text().collect::<Vec<_>>().join(" ");
                let cleaned: Vec<&str> = text
                    .split_whitespace()
                    .collect();
                if cleaned.len() > 50 {
                    return cleaned.join(" ");
                }
            }
        }
    }

    document
        .select(&Selector::parse("body").unwrap())
        .next()
        .map(|body| {
            body.text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}
