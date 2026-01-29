//! x402 Fetch tool for making paid HTTP requests via x402 protocol

use crate::tools::registry::Tool;
use crate::tools::types::{
    PropertySchema, ToolContext, ToolDefinition, ToolGroup, ToolInputSchema, ToolResult,
};
use crate::x402::X402Client;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

/// x402 Fetch tool for paid HTTP requests
pub struct X402FetchTool {
    definition: ToolDefinition,
}

impl X402FetchTool {
    pub fn new() -> Self {
        let mut properties = HashMap::new();

        properties.insert(
            "url".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "The URL to fetch. Must be an x402-enabled endpoint (e.g., quoter.defirelay.com, rpc.defirelay.com)".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "method".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "HTTP method: 'GET' or 'POST'".to_string(),
                default: Some(json!("GET")),
                items: None,
                enum_values: Some(vec!["GET".to_string(), "POST".to_string()]),
            },
        );

        properties.insert(
            "body".to_string(),
            PropertySchema {
                schema_type: "object".to_string(),
                description: "Request body for POST requests (JSON object)".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        properties.insert(
            "jq_filter".to_string(),
            PropertySchema {
                schema_type: "string".to_string(),
                description: "Optional jq-style filter to extract specific fields from the response. Examples: '.transaction', '.buyAmount', '{to: .transaction.to, data: .transaction.data}'".to_string(),
                default: None,
                items: None,
                enum_values: None,
            },
        );

        X402FetchTool {
            definition: ToolDefinition {
                name: "x402_fetch".to_string(),
                description: "Make HTTP requests to x402-enabled endpoints with automatic payment. Use for APIs like quoter.defirelay.com (swap quotes). Costs USDC per request (typically ~0.0001 USDC).".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties,
                    required: vec!["url".to_string()],
                },
                group: ToolGroup::Web,
            },
        }
    }

    /// Get or create the x402 client
    fn get_client(&self) -> Result<X402Client, String> {
        let private_key = crate::config::burner_wallet_private_key()
            .ok_or("BURNER_WALLET_BOT_PRIVATE_KEY environment variable not set")?;

        X402Client::new(&private_key)
    }

    /// Apply a simple jq-like filter to extract fields from JSON
    fn apply_jq_filter(&self, value: &Value, filter: &str) -> Result<Value, String> {
        let filter = filter.trim();

        // Handle object construction: {key: .field, key2: .field2}
        if filter.starts_with('{') && filter.ends_with('}') {
            let inner = &filter[1..filter.len()-1];
            let mut result = serde_json::Map::new();

            // Simple parsing of key: .field pairs
            for part in Self::split_object_fields(inner) {
                let part = part.trim();
                if let Some(colon_pos) = part.find(':') {
                    let key = part[..colon_pos].trim();
                    let field_path = part[colon_pos+1..].trim();
                    let extracted = self.extract_field(value, field_path)?;
                    result.insert(key.to_string(), extracted);
                }
            }

            return Ok(Value::Object(result));
        }

        // Handle simple field access: .field or .field.subfield
        self.extract_field(value, filter)
    }

    /// Split object fields handling nested braces
    fn split_object_fields(s: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in s.chars() {
            match c {
                '{' | '[' => {
                    depth += 1;
                    current.push(c);
                }
                '}' | ']' => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 => {
                    fields.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(c),
            }
        }

        if !current.trim().is_empty() {
            fields.push(current.trim().to_string());
        }

        fields
    }

    /// Extract a field from JSON using dot notation
    fn extract_field(&self, value: &Value, path: &str) -> Result<Value, String> {
        let path = path.trim();

        // Handle identity
        if path == "." {
            return Ok(value.clone());
        }

        // Remove leading dot if present
        let path = path.strip_prefix('.').unwrap_or(path);

        // Navigate through the path
        let mut current = value;
        for part in path.split('.') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            match current {
                Value::Object(map) => {
                    current = map.get(part).ok_or_else(|| format!("Field '{}' not found", part))?;
                }
                Value::Array(arr) => {
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index).ok_or_else(|| format!("Index {} out of bounds", index))?;
                    } else {
                        return Err(format!("Cannot access '{}' on array", part));
                    }
                }
                _ => return Err(format!("Cannot access '{}' on non-object", part)),
            }
        }

        Ok(current.clone())
    }
}

impl Default for X402FetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct X402FetchParams {
    url: String,
    #[serde(default = "default_method")]
    method: String,
    body: Option<Value>,
    jq_filter: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

#[async_trait]
impl Tool for X402FetchTool {
    fn definition(&self) -> ToolDefinition {
        self.definition.clone()
    }

    async fn execute(&self, params: Value, _context: &ToolContext) -> ToolResult {
        let params: X402FetchParams = match serde_json::from_value(params) {
            Ok(p) => p,
            Err(e) => return ToolResult::error(format!("Invalid parameters: {}", e)),
        };

        // Validate URL is an x402 endpoint
        if !crate::x402::is_x402_endpoint(&params.url) {
            return ToolResult::error(
                "URL must be an x402-enabled endpoint (e.g., quoter.defirelay.com, rpc.defirelay.com). Use exec tool for regular HTTP requests."
            );
        }

        // Validate method
        let method = params.method.to_uppercase();
        if method != "GET" && method != "POST" {
            return ToolResult::error("Method must be 'GET' or 'POST'");
        }

        // Get the x402 client
        let client = match self.get_client() {
            Ok(c) => c,
            Err(e) => return ToolResult::error(e),
        };

        log::info!("[x402_fetch] {} {}", method, params.url);

        // Make the request
        let response = match method.as_str() {
            "GET" => client.get_with_payment(&params.url).await,
            "POST" => {
                let body = params.body.unwrap_or(json!({}));
                client.post_with_payment(&params.url, &body).await
            }
            _ => unreachable!(),
        };

        let response = match response {
            Ok(r) => r,
            Err(e) => return ToolResult::error(format!("Request failed: {}", e)),
        };

        // Check HTTP status
        let status = response.response.status();
        if !status.is_success() {
            let body = response.response.text().await.unwrap_or_default();
            return ToolResult::error(format!("HTTP error {}: {}", status, body));
        }

        // Parse response body
        let body = match response.response.text().await {
            Ok(b) => b,
            Err(e) => return ToolResult::error(format!("Failed to read response: {}", e)),
        };

        // Try to parse as JSON
        let json_value: Result<Value, _> = serde_json::from_str(&body);

        // Build result
        let result_content = match (&json_value, &params.jq_filter) {
            (Ok(json), Some(filter)) => {
                // Apply jq filter
                match self.apply_jq_filter(json, filter) {
                    Ok(filtered) => serde_json::to_string_pretty(&filtered).unwrap_or_else(|_| body.clone()),
                    Err(e) => return ToolResult::error(format!("Filter error: {}", e)),
                }
            }
            (Ok(json), None) => {
                // Return formatted JSON
                serde_json::to_string_pretty(json).unwrap_or_else(|_| body.clone())
            }
            (Err(_), _) => {
                // Return raw body
                body
            }
        };

        // Build metadata
        let mut metadata = json!({
            "url": params.url,
            "method": method,
            "status": status.as_u16(),
            "wallet": client.wallet_address(),
        });

        // Add payment info if a payment was made
        if let Some(payment) = response.payment {
            metadata["payment"] = json!({
                "amount": payment.amount_formatted,
                "asset": payment.asset,
                "pay_to": payment.pay_to,
            });
        }

        ToolResult::success(result_content).with_metadata(metadata)
    }
}
