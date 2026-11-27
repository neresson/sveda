use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_stream::stream;
use eventsource_stream::{EventStreamError, Eventsource};
use futures_util::{Stream, StreamExt};
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE,
};
use reqwest::Client;
use serde_json::{json, Value};
use sveda_protocol::diagnostics::{identifier, log, Level};

use crate::http_diagnostics::{
    content_encoding, media_type, read_error, redacted_shape, response_fields,
};
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
        let mut builder = Client::builder();
        if crate::yandex_only() {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }
        let client = builder
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
        if crate::yandex_only() {
            return Err(LlmError::Other(
                "Remote embeddings disabled in yandex-ru profile".into(),
            ));
        }
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
        let headers = responses_headers(&spec.key, false)?;
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
            if crate::yandex_only() && (model.url != crate::YANDEX_BASE_URL || model.protocol != Protocol::Responses) {
                yield Err(LlmError::Other("Provider disabled in yandex-ru profile".into()));
                return;
            }
            if model.key.trim().is_empty() {
                yield Err(LlmError::missing_api_key());
                return;
            }

            let (url, body, headers) = match model.protocol {
                Protocol::Responses => (
                    join_endpoint(&model.url, "responses"),
                    responses_body_for(&model, &request),
                    match responses_headers(&model.key, model.url == crate::YANDEX_BASE_URL) {
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

            let upstream = match client.post(url).headers(headers)
                .header(ACCEPT, "text/event-stream")
                .header(ACCEPT_ENCODING, "identity")
                .json(&body)
                .build() {
                Ok(upstream) => upstream,
                Err(error) => {
                    yield Err(read_error(&error, "build_request", || Value::Null));
                    return;
                }
            };
            log(Level::Debug, "llm.http.request", || {
                let items = body["input"].as_array().or_else(|| body["messages"].as_array());
                let reasoning_items = match model.protocol {
                    Protocol::Responses => items.into_iter().flatten()
                        .filter(|item| item["type"] == "reasoning").count(),
                    Protocol::Anthropic => items.into_iter().flatten()
                        .filter_map(|item| item["content"].as_array()).flatten()
                        .filter(|block| block["type"] == "thinking").count(),
                };
                json!({
                    "model": identifier(&model.id),
                    "api_model": identifier(&model.api_model),
                    "protocol": format!("{:?}", model.protocol),
                    "url_host": upstream.url().host_str(),
                    "body_bytes": upstream.body().and_then(reqwest::Body::as_bytes).map(<[u8]>::len),
                    "message_count": request.messages.len(),
                    "item_count": items.map(Vec::len),
                    "tool_count": request.tools.len(),
                    "tool_result_count": request.messages.iter().filter(|message| message.role == "tool").count(),
                    "thinking": request.thinking,
                    "reasoning_replay_items": reasoning_items,
                })
            });
            let started = Instant::now();
            let response = match client.execute(upstream).await {
                Ok(response) => response,
                Err(error) => {
                    yield Err(read_error(&error, "headers", || json!({
                        "model": identifier(&model.id), "elapsed_ms": started.elapsed().as_millis(),
                    })));
                    return;
                }
            };

            let mut meta = response_fields(&response, &model.key, started);
            meta["model"] = json!(identifier(&model.id));
            meta["protocol"] = json!(format!("{:?}", model.protocol));
            log(Level::Debug, "llm.http.response", || meta.clone());
            if !response.status().is_success() {
                yield Err(http_status_error(response, meta, started).await);
                return;
            }
            let encoding = content_encoding(response.headers());
            let media = media_type(response.headers());
            if !matches!(encoding, "" | "identity")
                || (!media.is_empty() && !media.eq_ignore_ascii_case("text/event-stream"))
            {
                log(Level::Warn, "llm.http.unexpected_response", || meta.clone());
                yield Err(LlmError::Other(
                    "The model provider returned an unexpected response format.".into(),
                ));
                return;
            }

            let mut parser = ProviderParser::new(model.protocol);
            let stats = Arc::new(Mutex::new(StreamStats::default()));
            let tracker = stats.clone();
            let mut events = response.bytes_stream().inspect(move |chunk| {
                if let Ok(chunk) = chunk {
                    let mut stats = tracker.lock().expect("stream stats");
                    stats.chunks += 1;
                    stats.bytes += chunk.len();
                    stats.last = chunk.clone();
                }
            }).eventsource();
            let mut event_count = 0;
            let totals = |events: usize| {
                let stats = stats.lock().expect("stream stats");
                json!({
                    "response": meta, "chunks": stats.chunks, "bytes": stats.bytes, "events": events,
                    "elapsed_ms": started.elapsed().as_millis(),
                })
            };
            while let Some(event) = events.next().await {
                let event = match event {
                    Ok(event) => event,
                    Err(EventStreamError::Transport(error)) => {
                        yield Err(read_error(&error, "sse_body", || totals(event_count)));
                        return;
                    }
                    Err(error) => {
                        let kind = match error {
                            EventStreamError::Utf8(_) => "utf8",
                            _ => "sse",
                        };
                        log(Level::Warn, "llm.http.parse_error", || {
                            let sample = stats.lock().expect("stream stats").last.clone();
                            json!({"kind": kind, "sample": redacted_shape(&sample), "stream": totals(event_count)})
                        });
                        yield Err(LlmError::Other("The model provider sent an unreadable event stream.".into()));
                        return;
                    }
                };
                event_count += 1;
                let chunks = match parser.try_push(&event.event, &event.data) {
                    Ok(chunks) => chunks,
                    Err(error) => {
                        log(Level::Warn, "llm.http.parse_error", || json!({
                            "kind": "json", "line": error.line(), "column": error.column(),
                            "sample": redacted_shape(event.data.as_bytes()), "stream": totals(event_count),
                        }));
                        yield Err(LlmError::Other("The model provider sent an unreadable event.".into()));
                        return;
                    }
                };
                for chunk in chunks {
                    if let LlmChunk::End { finish_reason } = &chunk {
                        if finish_reason == "error" {
                            log(Level::Warn, "llm.http.provider_error", || totals(event_count));
                            yield Err(LlmError::Other("The model provider reported that generation failed.".into()));
                        } else {
                            log(Level::Debug, "llm.http.completed", || totals(event_count));
                            yield Ok(chunk);
                        }
                        return;
                    }
                    yield Ok(chunk);
                }
            }
            log(Level::Warn, "llm.http.incomplete_stream", || totals(event_count));
            yield Err(LlmError::Other("The model provider ended the stream before completing the response.".into()));
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

#[derive(Default)]
struct StreamStats {
    chunks: usize,
    bytes: usize,
    last: bytes::Bytes,
}

async fn http_status_error(
    mut response: reqwest::Response,
    meta: Value,
    started: Instant,
) -> LlmError {
    const MAX_ERROR_BYTES: usize = 4096;
    let status = response.status().as_u16();
    let mut body = Vec::new();
    while body.len() < MAX_ERROR_BYTES {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                body.extend_from_slice(&chunk[..chunk.len().min(MAX_ERROR_BYTES - body.len())])
            }
            Ok(None) => break,
            Err(error) => {
                read_error(&error, "http_error_body", || {
                    json!({
                        "response": meta,
                        "bytes": body.len(), "elapsed_ms": started.elapsed().as_millis(),
                    })
                });
                break;
            }
        }
    }
    let text = String::from_utf8_lossy(&body);
    let error = LlmError::from_status(status, &text);
    log(Level::Warn, "llm.http.status_error", || {
        json!({
            "response": meta,
            "body_bytes_sampled": body.len(), "error_kind": error.kind(),
            "elapsed_ms": started.elapsed().as_millis(),
        })
    });
    log(Level::Debug, "llm.http.error_body_shape", || {
        let signals = [
            "reasoning_content",
            "reasoning_text",
            "encrypted_content",
            "tool_call_id",
            "context_length_exceeded",
            "invalid_request_error",
            "request too large",
        ]
        .into_iter()
        .filter(|signal| text.contains(signal))
        .collect::<Vec<_>>();
        json!({
            "sample": redacted_shape(&body),
            "signals": signals,
        })
    });
    if matches!(error, LlmError::Other(_)) {
        LlmError::Other(format!("The model provider returned HTTP {status}."))
    } else {
        error
    }
}

fn responses_headers(key: &str, yandex: bool) -> Result<HeaderMap, LlmError> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let value = if yandex {
        headers.insert("x-data-logging-enabled", HeaderValue::from_static("false"));
        format!("Api-Key {key}")
    } else {
        format!("Bearer {key}")
    };
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

#[cfg(test)]
mod yandex_tests {
    use super::*;
    use crate::{Catalog, YANDEX_BASE_URL};

    #[test]
    fn yandex_profile_pins_model_and_disables_logging_and_fallback() {
        let catalog = Catalog::yandex("test-folder", "fixture".into());
        let model = catalog.default().unwrap();
        assert_eq!(
            join_endpoint(&model.url, "responses"),
            format!("{YANDEX_BASE_URL}/responses")
        );
        assert_eq!(model.api_model, "gpt://test-folder/deepseek-v4.1-flash");
        assert_eq!(catalog.stream_chain(model).len(), 1);
        assert!(catalog.find("deepseek-v4-pro").is_none());
        assert!(!model.uses_plaintext_reasoning());
        let body = responses_body_for(
            model,
            &StepRequest {
                messages: vec![crate::ChatMessage {
                    role: "user".into(),
                    content: "test".into(),
                    ..Default::default()
                }],
                instructions: None,
                thinking: false,
                tools: vec![crate::ToolSpec {
                    name: "get_demo".into(),
                    description: "fixture".into(),
                    parameters: json!({"type": "object"}),
                }],
            },
        );
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);
        assert_eq!(body["tools"][0]["type"], "function");
        let headers = responses_headers(&model.key, true).unwrap();
        assert_eq!(headers[AUTHORIZATION], "Api-Key fixture");
        assert_eq!(headers["x-data-logging-enabled"], "false");
        assert!(Catalog::yandex("", "fixture".into())
            .default()
            .unwrap()
            .key
            .is_empty());
    }
}
