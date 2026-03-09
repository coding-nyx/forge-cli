use crate::providers::base::{ChatChunk, ChatRequest, ForgeProvider, ProviderError, Role};
use async_trait::async_trait;
use futures_util::stream::{self, BoxStream};
use tokio::time::{sleep, Duration};

pub struct MockProvider;

#[async_trait]
impl ForgeProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec!["mock-gpt-4".to_string(), "mock-llama-3".to_string()])
    }

    async fn chat(&self, request: ChatRequest) -> Result<String, ProviderError> {
        // Simple logic: If last message was tool output, say finished. Otherwise run ls.
        if let Some(last_msg) = request.messages.last() {
            if last_msg.content.starts_with("Tool Output") {
                return Ok("I have finished checking the directory.".to_string());
            }
        }
        Ok("I'll check the directory: <shell>ls</shell>".to_string())
    }

    async fn stream_chat(&self, request: ChatRequest) -> Result<BoxStream<'static, Result<ChatChunk, ProviderError>>, ProviderError> {
        let is_finished = request.messages.last()
            .map(|m| m.content.starts_with("Tool Output"))
            .unwrap_or(false);

        let tokens = if is_finished {
            vec!["I ", "have ", "finished ", "checking ", "the ", "directory."]
        } else {
            vec!["I'll ", "check ", "the ", "directory: ", "<shell>ls</shell>"]
        };

        let s = async_stream::stream! {
            for token in tokens {
                sleep(Duration::from_millis(50)).await;
                yield Ok(ChatChunk { text: token.to_string() });
            }
        };

        Ok(Box::pin(s))
    }
}
