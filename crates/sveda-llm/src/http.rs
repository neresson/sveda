use std::future::Future;
use std::pin::Pin;

use async_stream::stream;
use eventsource_stream::Eventsource;
use futures_util::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;

use crate::parse::ProviderParser;
use crate::{
    anthropic_body_for, join_endpoint, responses_body_for, LlmChunk, LlmClient, LlmError,
    ModelSpec, Protocol, StepRequest,
};

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn from_timeout(seconds: u64) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(seconds.max(1)))
            .build()
            .expect("reqwest client");
        Self { client }
    }
}

impl LlmClient for HttpClient {
    fn stream_step(
        &self,
        model: &ModelSpec,
        request: StepRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<LlmChunk, LlmError>> + Send>> {
        let client = self.client.clone();
        let model = model.clone();
        Box::pin(stream! {
            if model.key.trim().is_empty() {
                yield Err(LlmError::missing_api_key());
                return;
            }

            let (url, body, headers) = match model.protocol {
                Protocol::Responses => (
                    join_endpoint(&model.url, "responses"),
                    responses_body_for(&model, &request),
                    match responses_headers(&model.key) {
                        Ok(headers) => headers,
                        Err(error) => {
                            yield Err(error);
                            return;
                        }
                    },
                ),
                Protocol::Anthropic => (
                    join_endpoint(&model.url, "messages"),
                    anthropic_body_for(&model, &request),
                    match anthropic_headers(&model.key) {
                        Ok(headers) => headers,
                        Err(error) => {
                            yield Err(error);
                            return;
                        }
                    },
                ),
            };

            let response = match client.post(url).headers(headers).json(&body).send().await {
                Ok(response) => response,
                Err(error) => {
                    yield Err(LlmError::Other(error.to_string()));
                    return;
                }
            };

            let status = response.status().as_u16();
            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                yield Err(LlmError::from_status(status, &body));
                return;
            }

            let mut parser = ProviderParser::new(model.protocol);
            let mut events = response.bytes_stream().eventsource();
            while let Some(event) = events.next().await {
                let event = match event {
                    Ok(event) => event,
                    Err(error) => {
                        yield Err(LlmError::Other(error.to_string()));
                        return;
                    }
                };
                for chunk in parser.push(&event.event, &event.data) {
                    yield Ok(chunk);
                }
            }
        })
    }

    fn complete_text(
        &self,
        model: &ModelSpec,
        instructions: &str,
        prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, LlmError>> + Send>> {
        crate::collect_text(
            std::sync::Arc::new(self.clone()) as std::sync::Arc<dyn LlmClient>,
            model.clone(),
            instructions.to_string(),
            prompt.to_string(),
        )
    }
}

fn responses_headers(key: &str) -> Result<HeaderMap, LlmError> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let value = format!("Bearer {key}");
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&value).map_err(|error| LlmError::Other(error.to_string()))?,
    );
    Ok(headers)
}

fn anthropic_headers(key: &str) -> Result<HeaderMap, LlmError> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(key).map_err(|error| LlmError::Other(error.to_string()))?,
    );
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    Ok(headers)
}
