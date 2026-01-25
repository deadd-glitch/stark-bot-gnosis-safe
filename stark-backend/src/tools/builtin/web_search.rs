use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Web search tool using search APIs (Brave, SerpAPI, etc.)
pub struct WebSearchTool {
    definition: ToolDefinition,
}

impl WebSearchTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "query".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The search query".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "num_results".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Number of results to return (default: 5, max: 10)".to_string(),
                default: Some(json!(5)),
                items: None,
                enum_values: None,
            },
        );

        WebSearchTool {
            definition: ToolDefinition {
                name: "web_search".to_string(),
                description: "Search the web for information. Returns a list of relevant web pages with titles, URLs, and snippets.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["query".to_string()],
                },
                group: ToolGroup::Web,
            },
        }
    }
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct WebSearchParams {
    query: String,
    num_results: Option<u32>,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

// Brave Search API response structures
#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title: String,
    url: String,
    description: String,
}

// SerpAPI response structures
#[derive(Debug, Deserialize)]
struct SerpApiResponse {
    organic_results: Option<Vec<SerpResult>>,
}

#[derive(Debug, Deserialize)]
struct SerpResult {
    title: String,
    link: String,
    snippet: Option<String>,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let params: WebSearchParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let num_results = params.num_results.unwrap_or(5).min(10);

        // Try different search API providers
        // Check for Brave Search API key
        if let Ok(api_key) = std::env::var("BRAVE_SEARCH_API_KEY") {
            return self
                .search_brave(&params.query, num_results, &api_key)
                .await;
        }

        // Check for SerpAPI key
        if let Ok(api_key) = std::env::var("SERPAPI_API_KEY") {
            return self
                .search_serpapi(&params.query, num_results, &api_key)
                .await;
        }

        ToolResult::error(
            "No search API configured. Set BRAVE_SEARCH_API_KEY or SERPAPI_API_KEY environment variable.",
        )
    }
}

impl WebSearchTool {
    async fn search_brave(&self, query: &str, num_results: u32, api_key: &str) -> ToolResult {
        let client = reqwest::Client::new();
        let url = format!(
            "https://api.search.brave.com/res/v1/web/search?q={}&count={}",
            urlencoding::encode(query),
            num_results
        );

        let response = match client
            .get(&url)
            .header("X-Subscription-Token", api_key)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Failed to search: {}", e)),
        };

        if !response.status().is_success() {
            return ToolResult::error(format!(
                "Search API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let data: BraveSearchResponse = match response.json().await {
            Ok(d) => d,
            Err(e) => return ToolResult::error(format!("Failed to parse search results: {}", e)),
        };

        let results: Vec<SearchResult> = data
            .web
            .map(|w| {
                w.results
                    .into_iter()
                    .map(|r| SearchResult {
                        title: r.title,
                        url: r.url,
                        snippet: r.description,
                    })
                    .collect()
            })
            .unwrap_or_default();

        if results.is_empty() {
            return ToolResult::success("No results found for the query.");
        }

        let formatted = results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}\n   URL: {}\n   {}", i + 1, r.title, r.url, r.snippet))
            .collect::<Vec<_>>()
            .join("\n\n");

        ToolResult::success(formatted).with_metadata(json!({ "results": results }))
    }

    async fn search_serpapi(&self, query: &str, num_results: u32, api_key: &str) -> ToolResult {
        let client = reqwest::Client::new();
        let url = format!(
            "https://serpapi.com/search.json?q={}&api_key={}&num={}",
            urlencoding::encode(query),
            api_key,
            num_results
        );

        let response = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Failed to search: {}", e)),
        };

        if !response.status().is_success() {
            return ToolResult::error(format!(
                "Search API error: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let data: SerpApiResponse = match response.json().await {
            Ok(d) => d,
            Err(e) => return ToolResult::error(format!("Failed to parse search results: {}", e)),
        };

        let results: Vec<SearchResult> = data
            .organic_results
            .map(|r| {
                r.into_iter()
                    .map(|sr| SearchResult {
                        title: sr.title,
                        url: sr.link,
                        snippet: sr.snippet.unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        if results.is_empty() {
            return ToolResult::success("No results found for the query.");
        }

        let formatted = results
            .iter()
            .enumerate()
            .map(|(i, r)| format!("{}. {}\n   URL: {}\n   {}", i + 1, r.title, r.url, r.snippet))
            .collect::<Vec<_>>()
            .join("\n\n");

        ToolResult::success(formatted).with_metadata(json!({ "results": results }))
    }
}

// URL encoding helper
mod urlencoding {
    pub fn encode(s: &str) -> String {
        let mut encoded = String::new();
        for c in s.chars() {
            match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => encoded.push(c),
                ' ' => encoded.push_str("%20"),
                _ => {
                    for b in c.to_string().as_bytes() {
                        encoded.push_str(&format!("%{:02X}", b));
                    }
                }
            }
        }
        encoded
    }
}
