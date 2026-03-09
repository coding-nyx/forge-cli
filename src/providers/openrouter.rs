use crate::providers::base::{ChatChunk, ChatRequest, ForgeProvider, ProviderError, Role};
use crate::providers::utils::parse_sse_stream;
use async_trait::async_trait;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};
use reqwest::{Client, Response, StatusCode};

pub struct OpenRouterProvider {
    api_key: String,
    client: Client,
}

#[derive(Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Deserialize)]
struct MessageContent {
    content: String,
}

#[derive(Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: String,
}

// Structs for model listing
#[derive(Deserialize)]
struct OpenRouterModelList {
    data: Vec<OpenRouterModel>,
}

#[derive(Deserialize)]
struct OpenRouterModel {
    id: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
        }
    }

    async fn handle_error(&self, response: Response) -> ProviderError {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        
        if let Ok(error_res) = serde_json::from_str::<ApiErrorResponse>(&body_text) {
            let msg = error_res.error.message;
            match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::Auth(msg),
                StatusCode::NOT_FOUND => ProviderError::ModelNotFound(msg),
                StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimit,
                _ => ProviderError::ApiError(msg),
            }
        } else {
            ProviderError::ApiError(format!("HTTP {} - {}", status, body_text))
        }
    }
}

#[async_trait]
impl ForgeProvider for OpenRouterProvider {
    fn name(&self) -> &str {
        "openrouter"
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self.client
            .get("https://openrouter.ai/api/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let res_body: OpenRouterModelList = response.json().await?;
        Ok(res_body.data.into_iter().map(|m| m.id).collect())
    }

    async fn chat(&self, request: ChatRequest) -> Result<String, ProviderError> {
        let model = request.model.unwrap_or_else(|| "openai/gpt-3.5-turbo".to_string());
        let messages = request.messages.into_iter().map(|m| OpenRouterMessage {
            role: match m.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: m.content,
        }).collect();

        let req_body = OpenRouterRequest { model, messages, stream: false };

        let response = self.client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/forge-cli/forge-cli") 
            .json(&req_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let res_body: OpenRouterResponse = response.json().await?;
        Ok(res_body.choices.get(0).map(|c| c.message.content.clone()).unwrap_or_default())
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError> {
        let model = request.model.unwrap_or_else(|| "openai/gpt-3.5-turbo".to_string());
        let messages = request.messages.into_iter().map(|m| OpenRouterMessage {
            role: match m.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: m.content,
        }).collect();

        let req_body = OpenRouterRequest { model, messages, stream: true };

        let response = self.client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://github.com/forge-cli/forge-cli") 
            .json(&req_body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        Ok(parse_sse_stream(Box::pin(response.bytes_stream())))
    }
}
