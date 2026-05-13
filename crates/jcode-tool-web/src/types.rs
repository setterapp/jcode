use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchResult {
    pub url: String,
    pub content: String,
    pub content_type: String,
    pub status: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchQuery {
    pub query: String,
    pub max_results: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchEngine {
    Exa,
    Serper,
    Google,
}
