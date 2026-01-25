use crate::ai::types::{
    AiResponse, ClaudeContentBlock, ClaudeMessage as TypedClaudeMessage,
    ClaudeMessageContent, ClaudeTool, ToolCall, ToolResponse,
};
use crate::ai::{Message, MessageRole};
use crate::tools::ToolDefinition;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ClaudeClient {
    client: Client,
    endpoint: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct ClaudeCompletionRequest {
    model: String,
    messages: Vec<SimpleClaudeMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Debug, Serialize)]
struct SimpleClaudeMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ClaudeToolRequest {
    model: String,
    messages: Vec<TypedClaudeMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ClaudeTool>>,
}

#[derive(Debug, Deserialize)]
struct ClaudeCompletionResponse {
    content: Vec<ClaudeResponseContent>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponseContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ClaudeErrorResponse {
    error: ClaudeError,
}

#[derive(Debug, Deserialize)]
struct ClaudeError {
    message: String,
}

impl ClaudeClient {
    pub fn new(api_key: &str, endpoint: Option<&str>, model: Option<&str>) -> Result<Self, String> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let auth_value = header::HeaderValue::from_str(api_key)
            .map_err(|e| format!("Invalid API key format: {}", e))?;
        headers.insert("x-api-key", auth_value);
        headers.insert(
            "anthropic-version",
            header::HeaderValue::from_static("2023-06-01"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            endpoint: endpoint
                .unwrap_or("https://api.anthropic.com/v1/messages")
                .to_string(),
            model: model.unwrap_or("claude-sonnet-4-20250514").to_string(),
        })
    }

    pub async fn generate_text(&self, messages: Vec<Message>) -> Result<String, String> {
        // Extract system message if present
        let mut system_message = None;
        let filtered_messages: Vec<Message> = messages
            .into_iter()
            .filter(|m| {
                if m.role == MessageRole::System {
                    system_message = Some(m.content.clone());
                    false
                } else {
                    true
                }
            })
            .collect();

        let api_messages: Vec<SimpleClaudeMessage> = filtered_messages
            .into_iter()
            .map(|m| SimpleClaudeMessage {
                role: m.role.to_string(),
                content: m.content,
            })
            .collect();

        let request = ClaudeCompletionRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: 4096,
            system: system_message,
        };

        log::debug!("Sending request to Claude API: {:?}", request);

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Claude API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse the error response
            if let Ok(error_response) = serde_json::from_str::<ClaudeErrorResponse>(&error_text) {
                return Err(format!("Claude API error: {}", error_response.error.message));
            }

            return Err(format!(
                "Claude API returned error status: {}, body: {}",
                status, error_text
            ));
        }

        let response_data: ClaudeCompletionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Claude response: {}", e))?;

        // Concatenate all text content from response
        let content: String = response_data
            .content
            .iter()
            .filter(|c| c.content_type == "text")
            .filter_map(|c| c.text.clone())
            .collect();

        if content.is_empty() {
            return Err("Claude API returned no content".to_string());
        }

        Ok(content)
    }

    /// Generate a response with tool support
    pub async fn generate_with_tools(
        &self,
        messages: Vec<Message>,
        tool_messages: Vec<TypedClaudeMessage>,
        tools: Vec<ToolDefinition>,
    ) -> Result<AiResponse, String> {
        // Extract system message if present
        let mut system_message = None;
        let filtered_messages: Vec<Message> = messages
            .into_iter()
            .filter(|m| {
                if m.role == MessageRole::System {
                    system_message = Some(m.content.clone());
                    false
                } else {
                    true
                }
            })
            .collect();

        // Convert regular messages to typed messages
        let mut api_messages: Vec<TypedClaudeMessage> = filtered_messages
            .into_iter()
            .map(|m| TypedClaudeMessage {
                role: m.role.to_string(),
                content: ClaudeMessageContent::Text(m.content),
            })
            .collect();

        // Add tool messages (assistant tool_use + user tool_result pairs)
        api_messages.extend(tool_messages);

        // Convert tool definitions to Claude format
        let claude_tools: Vec<ClaudeTool> = tools
            .into_iter()
            .map(|t| ClaudeTool {
                name: t.name,
                description: t.description,
                input_schema: serde_json::to_value(t.input_schema).unwrap_or_default(),
            })
            .collect();

        let request = ClaudeToolRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: 4096,
            system: system_message,
            tools: if claude_tools.is_empty() {
                None
            } else {
                Some(claude_tools)
            },
        };

        log::debug!(
            "Sending tool request to Claude API: {}",
            serde_json::to_string_pretty(&request).unwrap_or_default()
        );

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Claude API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            if let Ok(error_response) = serde_json::from_str::<ClaudeErrorResponse>(&error_text) {
                return Err(format!("Claude API error: {}", error_response.error.message));
            }

            return Err(format!(
                "Claude API returned error status: {}, body: {}",
                status, error_text
            ));
        }

        let response_data: ClaudeCompletionResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Claude response: {}", e))?;

        // Parse the response content
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();

        for content in response_data.content {
            match content.content_type.as_str() {
                "text" => {
                    if let Some(text) = content.text {
                        text_content.push_str(&text);
                    }
                }
                "tool_use" => {
                    if let (Some(id), Some(name), Some(input)) =
                        (content.id, content.name, content.input)
                    {
                        tool_calls.push(ToolCall {
                            id,
                            name,
                            arguments: input,
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(AiResponse {
            content: text_content,
            tool_calls,
            stop_reason: response_data.stop_reason,
        })
    }

    /// Build tool result messages to continue conversation after tool execution
    pub fn build_tool_result_messages(
        tool_calls: &[ToolCall],
        tool_responses: &[ToolResponse],
    ) -> Vec<TypedClaudeMessage> {
        // First message: assistant with tool_use blocks
        let tool_use_blocks: Vec<ClaudeContentBlock> = tool_calls
            .iter()
            .map(|tc| ClaudeContentBlock::ToolUse {
                id: tc.id.clone(),
                name: tc.name.clone(),
                input: tc.arguments.clone(),
            })
            .collect();

        // Second message: user with tool_result blocks
        let tool_result_blocks: Vec<ClaudeContentBlock> = tool_responses
            .iter()
            .map(|tr| ClaudeContentBlock::tool_result(
                tr.tool_call_id.clone(),
                tr.content.clone(),
                tr.is_error,
            ))
            .collect();

        vec![
            TypedClaudeMessage::assistant_with_blocks(tool_use_blocks),
            TypedClaudeMessage::user_with_tool_results(tool_result_blocks),
        ]
    }
}
