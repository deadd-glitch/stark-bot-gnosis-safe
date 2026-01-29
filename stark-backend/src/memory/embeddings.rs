//! Embedding providers for vector search
//!
//! Supports multiple embedding providers:
//! - "openai" - OpenAI's text-embedding-ada-002 or text-embedding-3-small
//! - "local" - Local fastembed (future implementation)
//! - "none" - Disabled (fallback to BM25 only)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Configuration for embedding provider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider name: "openai", "local", or "none"
    pub provider: String,
    /// Model name (optional, uses provider default)
    pub model: Option<String>,
    /// API key (for remote providers)
    pub api_key: Option<String>,
    /// Batch size for embedding generation
    pub batch_size: usize,
    /// Embedding dimensions (depends on model)
    pub dimensions: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "none".to_string(),
            model: None,
            api_key: None,
            batch_size: 100,
            dimensions: 1536, // OpenAI default
        }
    }
}

impl EmbeddingConfig {
    pub fn openai(api_key: String) -> Self {
        Self {
            provider: "openai".to_string(),
            model: Some("text-embedding-3-small".to_string()),
            api_key: Some(api_key),
            batch_size: 100,
            dimensions: 1536,
        }
    }

    pub fn none() -> Self {
        Self::default()
    }

    pub fn is_enabled(&self) -> bool {
        self.provider != "none" && self.api_key.is_some()
    }
}

/// Result of generating an embedding
#[derive(Debug, Clone)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub model: String,
    pub dimensions: usize,
}

/// Trait for embedding providers
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Embedding, String>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, String>;

    /// Get the model name being used
    fn model_name(&self) -> &str;

    /// Get the embedding dimensions
    fn dimensions(&self) -> usize;
}

/// OpenAI embedding provider
pub struct OpenAIEmbedding {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAIEmbedding {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "text-embedding-3-small".to_string()),
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct OpenAIEmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
    model: String,
}

#[derive(Deserialize)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbedding {
    async fn embed(&self, text: &str) -> Result<Embedding, String> {
        let results = self.embed_batch(&[text]).await?;
        results.into_iter().next().ok_or_else(|| "No embedding returned".to_string())
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, String> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let request = OpenAIEmbeddingRequest {
            input: texts.iter().map(|s| s.to_string()).collect(),
            model: self.model.clone(),
        };

        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI API error {}: {}", status, body));
        }

        let result: OpenAIEmbeddingResponse = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        // Sort by index to maintain order
        let mut data = result.data;
        data.sort_by_key(|d| d.index);

        Ok(data.into_iter().map(|d| Embedding {
            dimensions: d.embedding.len(),
            vector: d.embedding,
            model: result.model.clone(),
        }).collect())
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        }
    }
}

/// No-op embedding provider (when vector search is disabled)
pub struct NoOpEmbedding;

#[async_trait]
impl EmbeddingProvider for NoOpEmbedding {
    async fn embed(&self, _text: &str) -> Result<Embedding, String> {
        Err("Embeddings disabled".to_string())
    }

    async fn embed_batch(&self, _texts: &[&str]) -> Result<Vec<Embedding>, String> {
        Err("Embeddings disabled".to_string())
    }

    fn model_name(&self) -> &str {
        "none"
    }

    fn dimensions(&self) -> usize {
        0
    }
}

/// Create an embedding provider based on configuration
pub fn create_provider(config: &EmbeddingConfig) -> Box<dyn EmbeddingProvider> {
    match config.provider.as_str() {
        "openai" if config.api_key.is_some() => {
            Box::new(OpenAIEmbedding::new(
                config.api_key.clone().unwrap(),
                config.model.clone(),
            ))
        }
        // Future: "local" provider using fastembed
        _ => Box::new(NoOpEmbedding),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.provider, "none");
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_config_openai() {
        let config = EmbeddingConfig::openai("test-key".to_string());
        assert_eq!(config.provider, "openai");
        assert!(config.is_enabled());
    }
}
