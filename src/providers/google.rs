use crate::providers::base::{ChatChunk, ChatRequest, ForgeProvider, ProviderError, Role};
use async_trait::async_trait;
use futures_util::stream::{BoxStream, StreamExt};
use serde::{Deserialize, Serialize};
use reqwest::{Client, Response, StatusCode};

pub struct GoogleProvider {
    api_key: String,
    client: Client,
}

#[derive(Serialize)]
struct GoogleRequest {
    contents: Vec<GoogleContent>,
    system_instruction: Option<GoogleContent>,
}

#[derive(Serialize)]
struct GoogleContent {
    role: String,
    parts: Vec<GooglePart>,
}

#[derive(Serialize)]
struct GooglePart {
    text: String,
}

#[derive(Deserialize)]
struct GoogleResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GoogleErrorDetail>,
}

#[derive(Deserialize, Clone)]
struct Candidate {
    content: GoogleContentResponse,
}

#[derive(Deserialize, Clone)]
struct GoogleContentResponse {
    parts: Vec<GooglePartResponse>,
}

#[derive(Deserialize, Clone)]
struct GooglePartResponse {
    text: String,
}

#[derive(Deserialize)]
struct GoogleErrorDetail {
    message: String,
    status: String,
}

impl GoogleProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            client: Client::new(),
        }
    }

    async fn handle_error(&self, response: Response) -> ProviderError {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        
        if let Ok(err_res) = serde_json::from_str::<GoogleResponse>(&body) {
            if let Some(err) = err_res.error {
                return match status {
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => ProviderError::Auth(err.message),
                    StatusCode::TOO_MANY_REQUESTS => ProviderError::RateLimit,
                    StatusCode::NOT_FOUND => ProviderError::ModelNotFound(err.message),
                    _ => ProviderError::ApiError(err.message),
                };
            }
        }
        ProviderError::ApiError(format!("Google HTTP {} - {}", status, body))
    }
}

#[async_trait]
impl ForgeProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    async fn chat(&self, request: ChatRequest) -> Result<String, ProviderError> {
        let model = request.model.unwrap_or_else(|| "gemini-1.5-flash".to_string());
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, self.api_key
        );

        let mut contents = Vec::new();
        let mut system_instruction = None;

        for msg in request.messages {
            match msg.role {
                Role::System => {
                    system_instruction = Some(GoogleContent {
                        role: "user".to_string(), 
                        parts: vec![GooglePart { text: msg.content }],
                    });
                }
                Role::User => {
                    contents.push(GoogleContent {
                        role: "user".to_string(),
                        parts: vec![GooglePart { text: msg.content }],
                    });
                }
                Role::Assistant => {
                    contents.push(GoogleContent {
                        role: "model".to_string(),
                        parts: vec![GooglePart { text: msg.content }],
                    });
                }
            }
        }

        let req_body = GoogleRequest { contents, system_instruction };
        let response = self.client.post(&url).json(&req_body).send().await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let res_body: GoogleResponse = response.json().await?;
        let text = res_body.candidates.and_then(|c| c.get(0).cloned())
            .and_then(|c| c.content.parts.get(0).cloned())
            .map(|p| p.text)
            .unwrap_or_default();
        
        Ok(text)
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError> {
        let model = request.model.unwrap_or_else(|| "gemini-1.5-flash".to_string());
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            model, self.api_key
        );

        let mut contents = Vec::new();
        let mut system_instruction = None;

        for msg in request.messages {
            match msg.role {
                Role::System => {
                    system_instruction = Some(GoogleContent {
                        role: "user".to_string(), 
                        parts: vec![GooglePart { text: msg.content }],
                    });
                }
                Role::User => {
                    contents.push(GoogleContent {
                        role: "user".to_string(),
                        parts: vec![GooglePart { text: msg.content }],
                    });
                }
                Role::Assistant => {
                    contents.push(GoogleContent {
                        role: "model".to_string(),
                        parts: vec![GooglePart { text: msg.content }],
                    });
                }
            }
        }

        let req_body = GoogleRequest { contents, system_instruction };
        let response = self.client.post(&url).json(&req_body).send().await?;

        if !response.status().is_success() {
            return Err(self.handle_error(response).await);
        }

        let mut stream = response.bytes_stream();
        let s = async_stream::stream! {
            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => { yield Err(ProviderError::Network(e)); continue; }
                };
                
                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(res) = serde_json::from_str::<GoogleResponse>(data) {
                            if let Some(candidates) = res.candidates {
                                if let Some(candidate) = candidates.get(0) {
                                    if let Some(part) = candidate.content.parts.get(0) {
                                        yield Ok(ChatChunk { text: part.text.clone() });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(s))
    }
}
