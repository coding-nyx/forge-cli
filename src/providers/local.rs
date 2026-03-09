use crate::providers::base::{ChatChunk, ChatRequest, ForgeProvider, ProviderError, Role};
use crate::providers::utils::parse_sse_stream;
use async_trait::async_trait;
use futures_util::stream::BoxStream;
use serde::{Deserialize, Serialize};
use reqwest::{Client, Response, StatusCode};

pub struct LocalProvider {
    base_url: String,
    client: Client,
}

#[derive(Serialize)]
struct LocalRequest {
    model: String,
    messages: Vec<LocalMessage>,
    stream: bool,
}

#[derive(Serialize)]
struct LocalMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct LocalResponse {
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

impl LocalProvider {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: Client::new(),
        }
    }

    async fn handle_error(&self, response: Response) -> ProviderError {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        match status {
            StatusCode::NOT_FOUND => ProviderError::ModelNotFound("Local server does not recognize the model or endpoint.".into()),
            StatusCode::UNAUTHORIZED => ProviderError::Auth("Local server required authentication which was not provided.".into()),
            _ => ProviderError::ApiError(format!("Local server error ({}): {}", status, body)),
        }
    }
}

#[async_trait]
impl ForgeProvider for LocalProvider {
    fn name(&self) -> &str {
        "local"
    }

    async fn chat(&self, request: ChatRequest) -> Result<String, ProviderError> {
        let model = request.model.unwrap_or_else(|| "llama3".to_string());
        let messages = request.messages.into_iter().map(|m| LocalMessage {
            role: match m.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: m.content,
        }).collect();

        let req_body = LocalRequest { model, messages, stream: false };
        let url = format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/'));

        let response = self.client.post(&url).json(&req_body).send().await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let res_body: LocalResponse = response.json().await?;
        Ok(res_body.choices.get(0).map(|c| c.message.content.clone()).unwrap_or_default())
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError> {
        let model = request.model.unwrap_or_else(|| "llama3".to_string());
        let messages = request.messages.into_iter().map(|m| LocalMessage {
            role: match m.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: m.content,
        }).collect();

        let req_body = LocalRequest { model, messages, stream: true };
        let url = format!("{}/v1/chat/completions", self.base_url.trim_end_matches('/'));

        let response = self.client.post(&url).json(&req_body).send().await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        Ok(parse_sse_stream(Box::pin(response.bytes_stream())))
    }
}
