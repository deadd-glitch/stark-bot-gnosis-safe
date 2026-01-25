use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a tool call made by the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool to call
    pub name: String,
    /// Arguments to pass to the tool as JSON
    pub arguments: Value,
}

/// Represents the result of a tool execution to send back to the AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    /// ID of the tool call this responds to
    pub tool_call_id: String,
    /// Content of the tool response
    pub content: String,
    /// Whether the tool execution resulted in an error
    pub is_error: bool,
}

impl ToolResponse {
    pub fn success(tool_call_id: String, content: String) -> Self {
        ToolResponse {
            tool_call_id,
            content,
            is_error: false,
        }
    }

    pub fn error(tool_call_id: String, error: String) -> Self {
        ToolResponse {
            tool_call_id,
            content: error,
            is_error: true,
        }
    }
}

/// Unified AI response that can contain both text and tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    /// Text content of the response (may be empty if only tool calls)
    pub content: String,
    /// Tool calls requested by the AI
    pub tool_calls: Vec<ToolCall>,
    /// The reason the AI stopped generating
    pub stop_reason: Option<String>,
}

impl AiResponse {
    pub fn text(content: String) -> Self {
        AiResponse {
            content,
            tool_calls: vec![],
            stop_reason: Some("end_turn".to_string()),
        }
    }

    pub fn with_tools(content: String, tool_calls: Vec<ToolCall>) -> Self {
        AiResponse {
            content,
            tool_calls,
            stop_reason: Some("tool_use".to_string()),
        }
    }

    /// Check if the response contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }

    /// Check if the AI wants to use tools
    pub fn is_tool_use(&self) -> bool {
        self.stop_reason.as_deref() == Some("tool_use") || !self.tool_calls.is_empty()
    }
}

/// Tool definition in Claude API format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// Content block types in Claude API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

impl ClaudeContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        ClaudeContentBlock::Text { text: text.into() }
    }

    pub fn tool_result(tool_use_id: String, content: String, is_error: bool) -> Self {
        ClaudeContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error: if is_error { Some(true) } else { None },
        }
    }
}

/// Message with tool content for Claude API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: ClaudeMessageContent,
}

/// Content can be either a string or array of content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudeMessageContent {
    Text(String),
    Blocks(Vec<ClaudeContentBlock>),
}

impl ClaudeMessage {
    pub fn user(content: impl Into<String>) -> Self {
        ClaudeMessage {
            role: "user".to_string(),
            content: ClaudeMessageContent::Text(content.into()),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        ClaudeMessage {
            role: "assistant".to_string(),
            content: ClaudeMessageContent::Text(content.into()),
        }
    }

    pub fn assistant_with_blocks(blocks: Vec<ClaudeContentBlock>) -> Self {
        ClaudeMessage {
            role: "assistant".to_string(),
            content: ClaudeMessageContent::Blocks(blocks),
        }
    }

    pub fn user_with_tool_results(results: Vec<ClaudeContentBlock>) -> Self {
        ClaudeMessage {
            role: "user".to_string(),
            content: ClaudeMessageContent::Blocks(results),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_response_text() {
        let response = AiResponse::text("Hello world".to_string());
        assert_eq!(response.content, "Hello world");
        assert!(response.tool_calls.is_empty());
        assert!(!response.is_tool_use());
    }

    #[test]
    fn test_ai_response_with_tools() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            name: "web_search".to_string(),
            arguments: serde_json::json!({"query": "rust programming"}),
        };
        let response = AiResponse::with_tools("Searching...".to_string(), vec![tool_call]);

        assert!(response.has_tool_calls());
        assert!(response.is_tool_use());
        assert_eq!(response.tool_calls.len(), 1);
    }

    #[test]
    fn test_tool_response() {
        let success = ToolResponse::success("call_123".to_string(), "Result".to_string());
        assert!(!success.is_error);

        let error = ToolResponse::error("call_456".to_string(), "Failed".to_string());
        assert!(error.is_error);
    }
}
