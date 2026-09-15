use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_stream::stream;
use futures_util::Stream;
use futures_util::StreamExt;
use serde_json::{json, Value};

pub use catalog::{Catalog, ModelSpec, Protocol, UnknownModelError};
pub use error::LlmError;
pub use http::HttpClient;
pub use scripted::ScriptedClient;

mod catalog;
mod error;
mod http;
mod parse;
mod scripted;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoredToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Vec<StoredToolCall>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone)]
pub struct StepRequest {
    pub messages: Vec<ChatMessage>,
    pub instructions: Option<String>,
    pub thinking: bool,
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LlmChunk {
    TextDelta(String),
    ReasoningDelta(String),
    ToolCall {
        id: String,
        name: String,
        input: Value,
    },
    Usage {
        prompt_tokens: u64,
        completion_tokens: u64,
    },
    End {
        finish_reason: String,
    },
}

pub trait LlmClient: Send + Sync {
    fn stream_step(
        &self,
        model: &ModelSpec,
        request: StepRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<LlmChunk, LlmError>> + Send>>;

    fn complete_text(
        &self,
        model: &ModelSpec,
        instructions: &str,
        prompt: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, LlmError>> + Send>>;
}

pub fn collect_text(
    llm: Arc<dyn LlmClient>,
    model: ModelSpec,
    instructions: String,
    prompt: String,
) -> Pin<Box<dyn Future<Output = Result<String, LlmError>> + Send>> {
    Box::pin(async move {
        let mut stream = llm.stream_step(
            &model,
            StepRequest {
                messages: vec![ChatMessage {
                    role: "user".into(),
                    content: prompt,
                    ..Default::default()
                }],
                instructions: Some(instructions).filter(|value| !value.is_empty()),
                thinking: false,
                tools: Vec::new(),
            },
        );
        let mut text = String::new();
        while let Some(item) = stream.next().await {
            match item? {
                LlmChunk::TextDelta(delta) => text.push_str(&delta),
                LlmChunk::ReasoningDelta(_)
                | LlmChunk::ToolCall { .. }
                | LlmChunk::Usage { .. }
                | LlmChunk::End { .. } => {}
            }
        }
        Ok(text.trim().to_string())
    })
}

pub fn stream_with_failover(
    llm: Arc<dyn LlmClient>,
    chain: Vec<ModelSpec>,
    request: StepRequest,
) -> Pin<Box<dyn Stream<Item = Result<LlmChunk, LlmError>> + Send>> {
    Box::pin(stream! {
        if chain.is_empty() {
            yield Err(LlmError::Other("no models in failover chain".into()));
            return;
        }

        let mut last_failoverable = None;
        let last_index = chain.len() - 1;
        for (index, model) in chain.into_iter().enumerate() {
            let mut inner = llm.stream_step(&model, request.clone());
            match inner.next().await {
                None => return,
                Some(Ok(chunk)) => {
                    yield Ok(chunk);
                    while let Some(item) = inner.next().await {
                        yield item;
                    }
                    return;
                }
                Some(Err(error)) if error.is_failoverable() && index < last_index => {
                    last_failoverable = Some(error);
                }
                Some(Err(error)) => {
                    yield Err(error);
                    return;
                }
            }
        }
        if let Some(error) = last_failoverable {
            yield Err(error);
        }
    })
}

pub fn responses_body(model: &str, request: &StepRequest) -> Value {
    let mut body = json!({
        "model": model,
        "input": map_responses_input(&request.messages),
        "stream": true,
        "store": false,
        "reasoning": {
            "effort": if request.thinking { "high" } else { "none" }
        }
    });
    if let Some(instructions) = request
        .instructions
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        body["instructions"] = json!(instructions);
    }
    if !request.tools.is_empty() {
        body["tools"] = json!(map_responses_tools(&request.tools));
        body["tool_choice"] = json!("auto");
    }
    body
}

pub fn anthropic_body(model: &str, request: &StepRequest) -> Value {
    let mut body = json!({
        "model": model,
        "messages": map_anthropic_messages(&request.messages),
        "max_tokens": 16384,
        "stream": true,
        "thinking": if request.thinking {
            json!({ "type": "enabled", "budget_tokens": 8192 })
        } else {
            json!({ "type": "disabled" })
        }
    });
    let system = request
        .instructions
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    if !request.tools.is_empty() {
        body["tools"] = json!(map_anthropic_tools(&request.tools));
        body["tool_choice"] = json!({ "type": "auto" });
    }
    body
}

fn map_responses_input(messages: &[ChatMessage]) -> Vec<Value> {
    let mut input = Vec::new();
    for message in messages {
        if message.role == "system" {
            continue;
        }
        if message.role == "tool" {
            if let Some(call_id) = &message.tool_call_id {
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": call_id,
                    "output": message.content,
                }));
            }
            continue;
        }
        if message.role == "assistant" && !message.tool_calls.is_empty() {
            if !message.content.trim().is_empty() {
                input.push(json!({
                    "role": "assistant",
                    "content": message.content,
                }));
            }
            for call in &message.tool_calls {
                input.push(json!({
                    "type": "function_call",
                    "call_id": call.id,
                    "name": call.name,
                    "arguments": call.arguments,
                }));
            }
            continue;
        }
        if message.content.trim().is_empty() {
            continue;
        }
        input.push(json!({
            "role": message.role,
            "content": message.content,
        }));
    }
    input
}

fn map_anthropic_messages(messages: &[ChatMessage]) -> Vec<Value> {
    let mut mapped = Vec::new();
    for message in messages {
        if message.role == "system" {
            continue;
        }
        if message.role == "tool" {
            let tool_result = json!({
                "type": "tool_result",
                "tool_use_id": message.tool_call_id.clone().unwrap_or_default(),
                "content": message.content,
            });
            if let Some(Value::Object(existing)) = mapped.last_mut() {
                if existing.get("role").and_then(Value::as_str) == Some("user") {
                    if let Some(Value::Array(content)) = existing.get_mut("content") {
                        content.push(tool_result);
                        continue;
                    }
                }
            }
            mapped.push(json!({
                "role": "user",
                "content": [tool_result],
            }));
            continue;
        }
        if message.role == "assistant" && !message.tool_calls.is_empty() {
            let mut content = Vec::new();
            if !message.content.trim().is_empty() {
                content.push(json!({ "type": "text", "text": message.content }));
            }
            for call in &message.tool_calls {
                let input = serde_json::from_str(&call.arguments).unwrap_or(json!({}));
                content.push(json!({
                    "type": "tool_use",
                    "id": call.id,
                    "name": call.name,
                    "input": input,
                }));
            }
            mapped.push(json!({
                "role": "assistant",
                "content": content,
            }));
            continue;
        }
        if message.content.trim().is_empty() {
            continue;
        }
        mapped.push(json!({
            "role": message.role,
            "content": message.content,
        }));
    }
    mapped
}

fn map_responses_tools(tools: &[ToolSpec]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters,
            })
        })
        .collect()
}

fn map_anthropic_tools(tools: &[ToolSpec]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": tool.parameters,
            })
        })
        .collect()
}

pub fn parse_provider_sse(protocol: Protocol, body: &str) -> Vec<LlmChunk> {
    parse::parse_provider_sse(protocol, body)
}

pub fn join_endpoint(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

pub(crate) fn queued_stream(
    chunks: Vec<Result<LlmChunk, LlmError>>,
) -> Pin<Box<dyn Stream<Item = Result<LlmChunk, LlmError>> + Send>> {
    let mut queue = VecDeque::from(chunks);
    Box::pin(stream! {
        while let Some(item) = queue.pop_front() {
            yield item;
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user(text: &str) -> StepRequest {
        StepRequest {
            messages: vec![ChatMessage {
                role: "user".into(),
                content: text.into(),
                ..Default::default()
            }],
            instructions: Some("You are Veda.".into()),
            thinking: true,
            tools: Vec::new(),
        }
    }

    #[test]
    fn responses_thinking_on_sets_effort_high() {
        let body = responses_body("deepseek-v4-flash", &user("Say hello"));
        assert_eq!(body["model"], "deepseek-v4-flash");
        assert_eq!(body["store"], false);
        assert_eq!(body["reasoning"]["effort"], "high");
        assert!(body.get("include").is_none());
        assert!(body["input"].is_array());
    }

    #[test]
    fn responses_tools_use_function_shape() {
        let mut request = user("Say hello");
        request.tools = vec![ToolSpec {
            name: "get_calendar_events".into(),
            description: "Get calendar events".into(),
            parameters: json!({ "type": "object", "properties": {} }),
        }];
        let body = responses_body("deepseek-v4-flash", &request);
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["name"], "get_calendar_events");
        assert!(body["tools"][0].get("function").is_none());
        let anthropic = anthropic_body("deepseek-v4-flash", &request);
        assert_eq!(anthropic["tools"][0]["name"], "get_calendar_events");
        assert!(anthropic["tools"][0].get("input_schema").is_some());
    }

    #[test]
    fn responses_thinking_off_sets_effort_none() {
        let mut request = user("Say hello");
        request.thinking = false;
        let body = responses_body("deepseek-v4-flash", &request);
        assert_eq!(body["reasoning"]["effort"], "none");
    }

    #[test]
    fn anthropic_thinking_on_sets_enabled_budget() {
        let body = anthropic_body("deepseek-v4-flash", &user("Say hello"));
        assert_eq!(body["thinking"]["type"], "enabled");
        assert_eq!(body["thinking"]["budget_tokens"], 8192);
        assert!(body["messages"].is_array());
        assert_eq!(body["system"], "You are Veda.");
    }

    #[test]
    fn anthropic_thinking_off_sets_disabled() {
        let mut request = user("Say hello");
        request.thinking = false;
        let body = anthropic_body("deepseek-v4-flash", &request);
        assert_eq!(body["thinking"]["type"], "disabled");
    }

    #[test]
    fn parses_responses_text_reasoning_and_usage() {
        let body = "\
event: response.reasoning_text.delta\n\
data: {\"type\":\"response.reasoning_text.delta\",\"delta\":\"plan\"}\n\
\n\
event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello\"}\n\
\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":12,\"output_tokens\":8}}}\n\
\n";
        let chunks = parse_provider_sse(Protocol::Responses, body);
        assert!(matches!(&chunks[0], LlmChunk::ReasoningDelta(text) if text == "plan"));
        assert!(matches!(&chunks[1], LlmChunk::TextDelta(text) if text == "Hello"));
        assert!(matches!(
            &chunks[2],
            LlmChunk::Usage {
                prompt_tokens: 12,
                completion_tokens: 8
            }
        ));
        assert!(matches!(&chunks[3], LlmChunk::End { finish_reason } if finish_reason == "stop"));
    }

    #[test]
    fn parses_anthropic_thinking_and_text() {
        let body = "\
event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":12}}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"plan\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\
\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":8}}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
\n";
        let chunks = parse_provider_sse(Protocol::Anthropic, body);
        assert!(matches!(&chunks[0], LlmChunk::ReasoningDelta(text) if text == "plan"));
        assert!(matches!(&chunks[1], LlmChunk::TextDelta(text) if text == "Hello"));
        assert!(matches!(
            &chunks[2],
            LlmChunk::Usage {
                prompt_tokens: 12,
                completion_tokens: 8
            }
        ));
        assert!(matches!(&chunks[3], LlmChunk::End { finish_reason } if finish_reason == "stop"));
    }

    fn spec(id: &str) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            label: id.into(),
            protocol: Protocol::Responses,
            api_model: id.into(),
            url: "https://example.test".into(),
            key: "k".into(),
            aliases: Vec::new(),
        }
    }

    #[tokio::test]
    async fn failover_tries_next_before_any_chunk() {
        let llm = Arc::new(ScriptedClient::queue(vec![
            vec![Err(LlmError::RateLimited)],
            vec![
                Ok(LlmChunk::TextDelta("ok".into())),
                Ok(LlmChunk::End {
                    finish_reason: "stop".into(),
                }),
            ],
        ]));
        let mut stream =
            stream_with_failover(llm, vec![spec("primary"), spec("backup")], user("hi"));
        let mut texts = Vec::new();
        while let Some(item) = stream.next().await {
            if let Ok(LlmChunk::TextDelta(text)) = item {
                texts.push(text);
            }
        }
        assert_eq!(texts, vec!["ok".to_string()]);
    }

    #[tokio::test]
    async fn failover_does_not_switch_after_first_chunk() {
        let llm = Arc::new(ScriptedClient::queue(vec![
            vec![
                Ok(LlmChunk::TextDelta("partial".into())),
                Err(LlmError::RateLimited),
            ],
            vec![Ok(LlmChunk::TextDelta("should-not-run".into()))],
        ]));
        let mut stream =
            stream_with_failover(llm, vec![spec("primary"), spec("backup")], user("hi"));
        let mut texts = Vec::new();
        let mut errors = 0;
        while let Some(item) = stream.next().await {
            match item {
                Ok(LlmChunk::TextDelta(text)) => texts.push(text),
                Err(_) => errors += 1,
                _ => {}
            }
        }
        assert_eq!(texts, vec!["partial".to_string()]);
        assert_eq!(errors, 1);
    }
}
