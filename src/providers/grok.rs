use crate::providers::base::{ChatChunk, ChatRequest, ForgeProvider, ProviderError, Role};
use crate::providers::utils::parse_sse_stream;
use async_trait::async_trait;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};
use reqwest::Client;

pub struct GrokProvider {
    api_key: String,
    client: Client,
}

#[derive(Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<GrokMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct GrokResponse {
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

impl GrokProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
        }
    }
}

#[async_trait]
impl ForgeProvider for GrokProvider {
    fn name(&self) -> &str {
        "grok"
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let response = self.client
            .get("https://api.x.ai/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("Grok API error: {}", err_text)));
        }

        #[derive(Deserialize)]
        struct GrokModelList { data: Vec<GrokModel> }
        #[derive(Deserialize)]
        struct GrokModel { id: String }

        let res_body: GrokModelList = response.json().await?;
        Ok(res_body.data.into_iter().map(|m| m.id).collect())
    }

    async fn chat(&self, request: ChatRequest) -> Result<String, ProviderError> {
        let model = request.model.unwrap_or_else(|| "grok-3".to_string());
        
        let messages = request.messages.into_iter().map(|m| GrokMessage {
            role: match m.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: m.content,
        }).collect();

        let req_body = GrokRequest { model, messages, stream: false };

        let response = self.client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("Grok API error: {}", err_text)));
        }

        let res_body: GrokResponse = response.json().await?;
        Ok(res_body.choices.get(0).map(|c| c.message.content.clone()).unwrap_or_default())
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError> {
        let model = request.model.unwrap_or_else(|| "grok-3".to_string());
        let messages = request.messages.into_iter().map(|m| GrokMessage {
            role: match m.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: m.content,
        }).collect();

        let req_body = GrokRequest { model, messages, stream: true };

        let response = self.client
            .post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!("Grok API error: {}", err_text)));
        }

        Ok(parse_sse_stream(Box::pin(response.bytes_stream())))
    }
}
