pub mod claude;
pub mod llama;
pub mod openai;
pub mod types;

pub use claude::ClaudeClient;
pub use llama::LlamaClient;
pub use openai::OpenAIClient;
pub use types::{AiResponse, ClaudeMessage as TypedClaudeMessage, ToolCall, ToolResponse};

use crate::models::{AgentSettings, AiProvider};
use crate::tools::ToolDefinition;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl ToString for MessageRole {
    fn to_string(&self) -> String {
        match self {
            MessageRole::System => "system".to_string(),
            MessageRole::User => "user".to_string(),
            MessageRole::Assistant => "assistant".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

/// Unified AI client that works with any configured provider
pub enum AiClient {
    Claude(ClaudeClient),
    OpenAI(OpenAIClient),
    Llama(LlamaClient),
}

impl AiClient {
    /// Create an AI client from agent settings
    pub fn from_settings(settings: &AgentSettings) -> Result<Self, String> {
        let provider = settings.provider_enum().ok_or_else(|| {
            format!("Unknown provider: {}", settings.provider)
        })?;

        match provider {
            AiProvider::Claude => {
                let client = ClaudeClient::new(
                    &settings.api_key,
                    Some(&settings.endpoint),
                    Some(&settings.model),
                )?;
                Ok(AiClient::Claude(client))
            }
            AiProvider::OpenAI | AiProvider::OpenAICompatible => {
                // Both OpenAI and OpenAI-compatible use the same client
                // The endpoint from settings is always used
                let client = OpenAIClient::new(
                    &settings.api_key,
                    Some(&settings.endpoint),
                    Some(&settings.model),
                )?;
                Ok(AiClient::OpenAI(client))
            }
            AiProvider::Llama => {
                let client = LlamaClient::new(
                    Some(&settings.endpoint),
                    Some(&settings.model),
                )?;
                Ok(AiClient::Llama(client))
            }
        }
    }

    /// Generate text using the configured provider
    pub async fn generate_text(&self, messages: Vec<Message>) -> Result<String, String> {
        match self {
            AiClient::Claude(client) => client.generate_text(messages).await,
            AiClient::OpenAI(client) => client.generate_text(messages).await,
            AiClient::Llama(client) => client.generate_text(messages).await,
        }
    }

    /// Generate response with tool support (currently only Claude supports tools)
    pub async fn generate_with_tools(
        &self,
        messages: Vec<Message>,
        tool_messages: Vec<TypedClaudeMessage>,
        tools: Vec<ToolDefinition>,
    ) -> Result<AiResponse, String> {
        match self {
            AiClient::Claude(client) => {
                client.generate_with_tools(messages, tool_messages, tools).await
            }
            // Other providers fall back to text-only generation
            AiClient::OpenAI(client) => {
                let text = client.generate_text(messages).await?;
                Ok(AiResponse::text(text))
            }
            AiClient::Llama(client) => {
                let text = client.generate_text(messages).await?;
                Ok(AiResponse::text(text))
            }
        }
    }

    /// Check if the current provider supports tools
    pub fn supports_tools(&self) -> bool {
        matches!(self, AiClient::Claude(_))
    }

    /// Build tool result messages for continuing after tool execution (Claude-specific)
    pub fn build_tool_result_messages(
        tool_calls: &[ToolCall],
        tool_responses: &[ToolResponse],
    ) -> Vec<TypedClaudeMessage> {
        ClaudeClient::build_tool_result_messages(tool_calls, tool_responses)
    }
}
