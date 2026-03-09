use async_trait::async_trait;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Authentication failed: {0}. Please check your API key in 'forge setup'.")]
    Auth(String),

    #[error("Model not found: {0}. The provider doesn't recognize this model name.")]
    ModelNotFound(String),

    #[error("Rate limit exceeded. Please wait a moment before trying again.")]
    RateLimit,

    #[error("Provider API returned an error: {0}")]
    ApiError(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatChunk {
    pub text: String,
}

#[async_trait]
pub trait ForgeProvider: Send + Sync {
    fn name(&self) -> &str { "unknown" }
    async fn chat(&self, request: ChatRequest) -> Result<String, ProviderError>;
    async fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError>;
    
    /// List available models for this provider
    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec![])
    }
}
