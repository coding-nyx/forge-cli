use crate::providers::base::{ChatChunk, ProviderError};
use futures_util::Stream;
use futures_util::StreamExt;
use serde::Deserialize;
use std::pin::Pin;

#[derive(Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
}

pub fn parse_sse_stream(
    mut stream: Pin<Box<dyn Stream<Item = reqwest::Result<bytes::Bytes>> + Send>>,
) -> Pin<Box<dyn Stream<Item = Result<ChatChunk, ProviderError>> + Send>> {
    let s = async_stream::stream! {
        while let Some(chunk_result) = stream.next().await {
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    yield Err(ProviderError::Network(e));
                    continue;
                }
            };

            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line == "data: [DONE]" {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(res) = serde_json::from_str::<StreamResponse>(data) {
                        if let Some(choice) = res.choices.get(0) {
                            if let Some(content) = &choice.delta.content {
                                yield Ok(ChatChunk { text: content.clone() });
                            }
                        }
                    }
                }
            }
        }
    };
    Box::pin(s)
}
