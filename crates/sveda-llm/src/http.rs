use std::future::Future;
use std::pin::Pin;

use async_stream::stream;
use eventsource_stream::Eventsource;
use futures_util::{Stream, StreamExt};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;

use crate::parse::ProviderParser;
use crate::{
    anthropic_body_for, clamp_embedding_input, join_endpoint, responses_body_for,
    supports_dimensions_param, EmbeddingSpec, LlmChunk, LlmClient, LlmError, ModelSpec, Protocol,
    StepRequest, EMBED_BATCH_SIZE,
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

    pub async fn embed(
        &self,
        spec: &EmbeddingSpec,
        inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, LlmError> {
        if spec.key.trim().is_empty() {
            return Err(LlmError::Other(
                "Missing API key for the selected embedding. Add it in Admin → Models.".into(),
            ));
        }
        let mut vectors = Vec::with_capacity(inputs.len());
        for batch in inputs.chunks(EMBED_BATCH_SIZE) {
            vectors.extend(self.embed_openai_batch(spec, batch).await?);
        }
        Ok(vectors)
    }

    async fn embed_openai_batch(
        &self,
        spec: &EmbeddingSpec,
        inputs: &[String],
    ) -> Result<Vec<Vec<f32>>, LlmError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let clipped: Vec<String> = inputs
            .iter()
            .map(|text| clamp_embedding_input(text))
            .collect();
        let mut body = serde_json::json!({
            "model": spec.api_model,
            "input": clipped,
        });
        if supports_dimensions_param(&spec.api_model) {
            body["dimensions"] = serde_json::json!(spec.dimensions);
        }
        let headers = responses_headers(&spec.key)?;
        let url = join_endpoint(&spec.url, "embeddings");
        let response = self
            .client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|error| LlmError::Other(error.to_string()))?;
        let status = response.status().as_u16();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| LlmError::Other(error.to_string()))?;
        if !(200..300).contains(&status) {
            let body = String::from_utf8_lossy(&bytes);
            return Err(LlmError::from_status(status, &body));
        }
        parse_openai_embeddings(&bytes, spec.dimensions, inputs.len())
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

fn parse_openai_embeddings(
    bytes: &[u8],
    expected_dims: usize,
    expected_count: usize,
) -> Result<Vec<Vec<f32>>, LlmError> {
    let payload: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| LlmError::Other(format!("embedding json: {error}")))?;
    let mut rows: Vec<(usize, Vec<f32>)> = payload
        .get("data")
        .and_then(|value| value.as_array())
        .ok_or_else(|| LlmError::Other("embedding response missing data".into()))?
        .iter()
        .enumerate()
        .map(|(fallback, item)| {
            let index = item
                .get("index")
                .and_then(|value| value.as_u64())
                .map(|value| value as usize)
                .unwrap_or(fallback);
            let embedding =
                item.get("embedding")
                    .and_then(|value| value.as_array())
                    .ok_or_else(|| LlmError::Other("embedding vector missing".into()))?
                    .iter()
                    .map(|value| {
                        value.as_f64().map(|value| value as f32).ok_or_else(|| {
                            LlmError::Other("embedding value is not a number".into())
                        })
                    })
                    .collect::<Result<Vec<f32>, LlmError>>()?;
            if embedding.len() != expected_dims {
                return Err(LlmError::Other(format!(
                    "embedding dimension mismatch: expected {expected_dims}, got {}",
                    embedding.len()
                )));
            }
            Ok((index, embedding))
        })
        .collect::<Result<Vec<_>, LlmError>>()?;
    rows.sort_by_key(|(index, _)| *index);
    let vectors: Vec<Vec<f32>> = rows.into_iter().map(|(_, vector)| vector).collect();
    if vectors.len() != expected_count {
        return Err(LlmError::Other(format!(
            "embedding count mismatch: expected {expected_count}, got {}",
            vectors.len()
        )));
    }
    Ok(vectors)
}
