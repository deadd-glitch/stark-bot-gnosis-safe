use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Web fetch tool to retrieve and parse content from URLs
pub struct WebFetchTool {
    definition: ToolDefinition,
}

impl WebFetchTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();
        properties.insert(
            "url".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The URL to fetch content from".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "max_length".to_string(),
            PropertySchema {
                schema_type: "integer".to_string(),
                description: "Maximum content length to return (default: 10000 characters)"
                    .to_string(),
                default: Some(json!(10000)),
                items: None,
                enum_values: None,
            },
        );
        properties.insert(
            "extract_text".to_string(),
            PropertySchema {
                schema_type: "boolean".to_string(),
                description: "If true, extract plain text from HTML (default: true)".to_string(),
                default: Some(json!(true)),
                items: None,
                enum_values: None,
            },
        );

        WebFetchTool {
            definition: ToolDefinition {
                name: "web_fetch".to_string(),
                description: "Fetch content from a URL. Can extract plain text from HTML pages or return raw content.".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["url".to_string()],
                },
                group: ToolGroup::Web,
            },
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct WebFetchParams {
    url: String,
    max_length: Option<usize>,
    extract_text: Option<bool>,
}

#[async_trait]
impl Tool for WebFetchTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let params: WebFetchParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        let max_length = params.max_length.unwrap_or(10000);
        let extract_text = params.extract_text.unwrap_or(true);

        // Validate URL
        if !params.url.starts_with("http://") && !params.url.starts_with("https://") {
            return ToolResult::error("URL must start with http:// or https://");
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("StarkBot/1.0 (Web Fetch Tool)")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let response = match client.get(&params.url).send().await {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Failed to fetch URL: {}", e)),
        };

        let status = response.status();
        if !status.is_success() {
            return ToolResult::error(format!("HTTP error: {} for URL: {}", status, params.url));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = match response.text().await {
            Ok(t) => t,
            Err(e) => return ToolResult::error(format!("Failed to read response body: {}", e)),
        };

        let original_length = body.len();

        let content = if extract_text && content_type.contains("text/html") {
            extract_text_from_html(&body)
        } else {
            body
        };

        // Truncate if necessary
        let truncated = content.len() > max_length;
        let final_content = if truncated {
            format!(
                "{}\n\n[Content truncated at {} characters. Original length: {} characters]",
                &content[..max_length],
                max_length,
                content.len()
            )
        } else {
            content
        };

        ToolResult::success(final_content).with_metadata(json!({
            "url": params.url,
            "content_type": content_type,
            "truncated": truncated,
            "original_length": original_length
        }))
    }
}

/// Simple HTML to text extraction
fn extract_text_from_html(html: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    let mut in_script = false;
    let mut in_style = false;
    let mut last_was_space = false;

    let html_lower = html.to_lowercase();
    let chars: Vec<char> = html.chars().collect();
    let chars_lower: Vec<char> = html_lower.chars().collect();

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        // Check for script/style tags
        if i + 7 < chars_lower.len() {
            let slice: String = chars_lower[i..i + 7].iter().collect();
            if slice == "<script" {
                in_script = true;
            }
            if slice == "</scrip" {
                in_script = false;
            }
        }
        if i + 6 < chars_lower.len() {
            let slice: String = chars_lower[i..i + 6].iter().collect();
            if slice == "<style" {
                in_style = true;
            }
            if slice == "</styl" {
                in_style = false;
            }
        }

        if c == '<' {
            in_tag = true;
            i += 1;
            continue;
        }

        if c == '>' {
            in_tag = false;
            // Add newline after certain tags
            if i >= 3 {
                let prev: String = chars_lower[i.saturating_sub(3)..i].iter().collect();
                if prev.contains("/p")
                    || prev.contains("br")
                    || prev.contains("/h")
                    || prev.contains("/li")
                    || prev.contains("/tr")
                    || prev.contains("/di")
                {
                    if !last_was_space {
                        text.push('\n');
                        last_was_space = true;
                    }
                }
            }
            i += 1;
            continue;
        }

        if !in_tag && !in_script && !in_style {
            // Handle HTML entities
            if c == '&' {
                let remaining: String = chars[i..].iter().take(10).collect();
                if remaining.starts_with("&nbsp;") {
                    text.push(' ');
                    i += 6;
                    continue;
                } else if remaining.starts_with("&amp;") {
                    text.push('&');
                    i += 5;
                    continue;
                } else if remaining.starts_with("&lt;") {
                    text.push('<');
                    i += 4;
                    continue;
                } else if remaining.starts_with("&gt;") {
                    text.push('>');
                    i += 4;
                    continue;
                } else if remaining.starts_with("&quot;") {
                    text.push('"');
                    i += 6;
                    continue;
                } else if remaining.starts_with("&#") {
                    // Numeric entity
                    if let Some(end) = remaining.find(';') {
                        if let Ok(code) = remaining[2..end].parse::<u32>() {
                            if let Some(ch) = char::from_u32(code) {
                                text.push(ch);
                                i += end + 1;
                                continue;
                            }
                        }
                    }
                }
            }

            // Normalize whitespace
            if c.is_whitespace() {
                if !last_was_space {
                    text.push(' ');
                    last_was_space = true;
                }
            } else {
                text.push(c);
                last_was_space = false;
            }
        }

        i += 1;
    }

    // Clean up the text
    text.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_html() {
        let html = r#"
        <html>
        <head><title>Test</title></head>
        <body>
            <h1>Hello World</h1>
            <p>This is a <b>test</b> paragraph.</p>
            <script>var x = 1;</script>
            <p>Second paragraph with &amp; entity.</p>
        </body>
        </html>
        "#;

        let text = extract_text_from_html(html);
        assert!(text.contains("Hello World"));
        assert!(text.contains("This is a test paragraph."));
        assert!(text.contains("&"));
        assert!(!text.contains("var x = 1"));
    }
}
