use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_stream::stream;
use futures_util::Stream;
use futures_util::StreamExt;
use serde_json::{json, Value};

pub use catalog::{Catalog, ModelSpec, Protocol, UnknownModelError};
pub use embeddings::{
    clamp_embedding_input, default_dimensions, default_usd_per_million, supports_dimensions_param,
    EmbeddingProtocol, EmbeddingSpec, DEFAULT_EMBEDDING_DIMS, DEFAULT_EMBEDDING_MODEL,
    EMBED_BATCH_SIZE, MAX_EMBEDDING_CHARS, MAX_EMBEDDING_DIMS, MIN_EMBEDDING_DIMS,
    OPENAI_EMBEDDINGS_URL,
};
pub use error::LlmError;
pub use http::HttpClient;
pub use scripted::ScriptedClient;

mod catalog;
mod embeddings;
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
    pub reasoning: String,
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
    responses_payload(model, request, false)
}

pub fn responses_body_for(spec: &ModelSpec, request: &StepRequest) -> Value {
    responses_payload(
        &spec.api_model,
        request,
        spec.uses_plaintext_reasoning() && !request.tools.is_empty(),
    )
}

pub fn anthropic_body_for(spec: &ModelSpec, request: &StepRequest) -> Value {
    anthropic_payload(
        &spec.api_model,
        request,
        spec.uses_plaintext_reasoning() && !request.tools.is_empty(),
    )
}

fn responses_payload(model: &str, request: &StepRequest, replay_reasoning: bool) -> Value {
    let mut body = json!({
        "model": model,
        "input": map_responses_input(&request.messages, replay_reasoning),
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
    anthropic_payload(model, request, false)
}

fn anthropic_payload(model: &str, request: &StepRequest, replay_thinking: bool) -> Value {
    let mut body = json!({
        "model": model,
        "messages": map_anthropic_messages(&request.messages, replay_thinking),
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

fn map_responses_input(messages: &[ChatMessage], replay_reasoning: bool) -> Vec<Value> {
    let mut input = Vec::new();
    for message in messages {
        match message.role.as_str() {
            "system" => continue,
            "tool" => {
                input.push(json!({
                    "type": "function_call_output",
                    "call_id": message.tool_call_id.clone().unwrap_or_default(),
                    "output": message.content,
                }));
            }
            "assistant" => {
                // DeepSeek requires a reasoning item on EVERY prior assistant turn
                // when `tools` is present, including text-only turns with no stored CoT.
                let mut replayed_reasoning = false;
                if replay_reasoning {
                    if let Some(item) = optional_responses_reasoning_item(&message.reasoning, true)
                    {
                        input.push(item);
                        replayed_reasoning = true;
                    }
                }
                if !message.content.trim().is_empty() {
                    input.push(json!({
                        "role": "assistant",
                        "content": message.content,
                    }));
                } else if replayed_reasoning && !message.tool_calls.is_empty() {
                    // Reasoning is merged into the adjacent assistant message.
                    // Tool-only turns have no visible text; omitting this item
                    // drops CoT and the next request 400s with reasoning_text.
                    input.push(json!({
                        "role": "assistant",
                        "content": "",
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
            }
            _ => {
                if message.content.trim().is_empty() {
                    continue;
                }
                input.push(json!({
                    "role": message.role,
                    "content": message.content,
                }));
            }
        }
    }
    pair_function_call_outputs(input)
}

fn optional_responses_reasoning_item(reasoning: &str, must_replay: bool) -> Option<Value> {
    let text = replay_reasoning_text(reasoning, must_replay);
    if text.is_empty() {
        None
    } else {
        Some(json!({
            "type": "reasoning",
            "content": [{ "type": "reasoning_text", "text": text }]
        }))
    }
}

fn replay_reasoning_text(reasoning: &str, must_replay: bool) -> String {
    let trimmed = reasoning.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }
    if must_replay {
        ".".into()
    } else {
        String::new()
    }
}

fn item_type(item: &Value) -> Option<&str> {
    item.get("type").and_then(Value::as_str)
}

fn call_id_of(item: &Value) -> Option<String> {
    item.get("call_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(ToOwned::to_owned)
}

fn take_function_call_output(
    id: &str,
    outputs: &mut HashMap<String, Value>,
    outputs_without_id: &mut Vec<Value>,
) -> Value {
    if let Some(output) = outputs.remove(id) {
        return output;
    }
    if !outputs_without_id.is_empty() {
        let mut output = outputs_without_id.remove(0);
        output["call_id"] = json!(id);
        return output;
    }
    json!({
        "type": "function_call_output",
        "call_id": id,
        "output": "Tool result was not available.",
    })
}

// DeepSeek merges reasoning + function_call items into the adjacent assistant
// message and always allows parallel tools. Interleaving output after each call
// splits later calls onto a new assistant turn without reasoning_text (HTTP 400).
fn pair_function_call_outputs(input: Vec<Value>) -> Vec<Value> {
    let mut outputs: HashMap<String, Value> = HashMap::new();
    let mut outputs_without_id = Vec::new();
    let mut rest = Vec::new();
    for item in input {
        if item_type(&item) != Some("function_call_output") {
            rest.push(item);
            continue;
        }
        match call_id_of(&item) {
            Some(id) => {
                outputs.insert(id, item);
            }
            None => outputs_without_id.push(item),
        }
    }
    let mut paired = Vec::new();
    let mut pending_calls = Vec::new();
    for item in rest {
        if item_type(&item) == Some("function_call") {
            pending_calls.push(item);
            continue;
        }
        flush_function_call_batch(
            &mut paired,
            &mut pending_calls,
            &mut outputs,
            &mut outputs_without_id,
        );
        paired.push(item);
    }
    flush_function_call_batch(
        &mut paired,
        &mut pending_calls,
        &mut outputs,
        &mut outputs_without_id,
    );
    paired.extend(outputs.into_values());
    paired.extend(outputs_without_id);
    paired
}

fn flush_function_call_batch(
    paired: &mut Vec<Value>,
    pending_calls: &mut Vec<Value>,
    outputs: &mut HashMap<String, Value>,
    outputs_without_id: &mut Vec<Value>,
) {
    if pending_calls.is_empty() {
        return;
    }
    let calls = std::mem::take(pending_calls);
    let ids: Vec<String> = calls.iter().filter_map(call_id_of).collect();
    paired.extend(calls);
    for id in ids {
        paired.push(take_function_call_output(&id, outputs, outputs_without_id));
    }
}

fn map_anthropic_messages(messages: &[ChatMessage], replay_thinking: bool) -> Vec<Value> {
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
        if message.role == "assistant" {
            let mut content = Vec::new();
            if replay_thinking {
                let thinking = replay_reasoning_text(&message.reasoning, true);
                if !thinking.is_empty() {
                    content.push(json!({
                        "type": "thinking",
                        "thinking": thinking,
                    }));
                }
            }
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
            if content.is_empty() {
                continue;
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
            instructions: Some("You are Sveda.".into()),
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
        assert_eq!(body["system"], "You are Sveda.");
    }

    #[test]
    fn anthropic_thinking_off_sets_disabled() {
        let mut request = user("Say hello");
        request.thinking = false;
        let body = anthropic_body("deepseek-v4-flash", &request);
        assert_eq!(body["thinking"]["type"], "disabled");
    }

    fn tool_followup(thinking: bool, reasoning: &str) -> StepRequest {
        StepRequest {
            messages: vec![
                ChatMessage {
                    role: "user".into(),
                    content: "что ты умеешь?".into(),
                    ..Default::default()
                },
                ChatMessage {
                    role: "assistant".into(),
                    content: String::new(),
                    reasoning: reasoning.into(),
                    tool_calls: vec![StoredToolCall {
                        id: "call-1".into(),
                        name: "search_agent_tools".into(),
                        arguments: "{}".into(),
                    }],
                    tool_call_id: None,
                },
                ChatMessage {
                    role: "tool".into(),
                    content: "{\"ok\":true}".into(),
                    tool_call_id: Some("call-1".into()),
                    ..Default::default()
                },
            ],
            instructions: Some("You are Sveda.".into()),
            thinking,
            tools: vec![ToolSpec {
                name: "search_agent_tools".into(),
                description: "Search tools".into(),
                parameters: json!({ "type": "object", "properties": {} }),
            }],
        }
    }

    fn openai_spec() -> ModelSpec {
        ModelSpec {
            id: "gpt-4o".into(),
            label: "GPT".into(),
            protocol: Protocol::Responses,
            api_model: "gpt-4o".into(),
            url: "https://api.openai.com/v1".into(),
            key: "k".into(),
            aliases: Vec::new(),
        }
    }

    fn claude_spec() -> ModelSpec {
        ModelSpec {
            id: "claude-sonnet".into(),
            label: "Claude".into(),
            protocol: Protocol::Anthropic,
            api_model: "claude-sonnet-4".into(),
            url: "https://api.anthropic.com".into(),
            key: "k".into(),
            aliases: Vec::new(),
        }
    }

    #[test]
    fn deepseek_specs_use_plaintext_reasoning() {
        let catalog = Catalog::builtin("k");
        for model in &catalog.models {
            assert!(
                model.uses_plaintext_reasoning(),
                "{} should replay plaintext reasoning",
                model.id
            );
        }
        assert!(!openai_spec().uses_plaintext_reasoning());
        assert!(!claude_spec().uses_plaintext_reasoning());
    }

    #[test]
    fn deepseek_responses_replay_reasoning_before_function_call() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let body = responses_body_for(&spec, &tool_followup(true, "check tools"));
        let input = body["input"].as_array().expect("input");
        assert_eq!(input[0]["role"], "user");
        assert_eq!(input[1]["type"], "reasoning");
        assert_eq!(input[1]["content"][0]["type"], "reasoning_text");
        assert_eq!(input[1]["content"][0]["text"], "check tools");
        assert!(input[1].get("summary").is_none());
        assert_eq!(input[2]["role"], "assistant");
        assert_eq!(input[2]["content"], "");
        assert_eq!(input[3]["type"], "function_call");
        assert_eq!(input[3]["call_id"], "call-1");
        assert_eq!(input[4]["type"], "function_call_output");
        assert_eq!(input[4]["call_id"], "call-1");
        assert_eq!(input[4]["output"], "{\"ok\":true}");
    }

    #[test]
    fn deepseek_replays_reasoning_when_tools_present_even_if_thinking_is_off() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let body = responses_body_for(&spec, &tool_followup(false, "check tools"));
        let input = body["input"].as_array().expect("input");
        assert_eq!(input[1]["type"], "reasoning");
        assert_eq!(input[1]["content"][0]["text"], "check tools");
        assert_eq!(input[2]["role"], "assistant");
        assert_eq!(input[3]["type"], "function_call");
    }

    #[test]
    fn deepseek_places_one_reasoning_item_before_text_and_parallel_calls() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let mut request = tool_followup(true, "check tools");
        request.messages[1].content = "I'll search".into();
        request.messages[1].tool_calls.push(StoredToolCall {
            id: "call-2".into(),
            name: "search_agent_tools".into(),
            arguments: "{\"q\":\"b\"}".into(),
        });
        request.messages.push(ChatMessage {
            role: "tool".into(),
            content: "second".into(),
            tool_call_id: Some("call-2".into()),
            ..Default::default()
        });
        let body = responses_body_for(&spec, &request);
        let input = body["input"].as_array().expect("input");
        let reasoning_count = input
            .iter()
            .filter(|item| item.get("type").and_then(Value::as_str) == Some("reasoning"))
            .count();
        assert_eq!(reasoning_count, 1);
        assert_eq!(input[1]["type"], "reasoning");
        assert_eq!(input[2]["role"], "assistant");
        assert_eq!(input[2]["content"], "I'll search");
        assert_eq!(input[3]["type"], "function_call");
        assert_eq!(input[3]["call_id"], "call-1");
        assert_eq!(input[4]["type"], "function_call");
        assert_eq!(input[4]["call_id"], "call-2");
        assert_eq!(input[5]["type"], "function_call_output");
        assert_eq!(input[5]["call_id"], "call-1");
        assert_eq!(input[6]["type"], "function_call_output");
        assert_eq!(input[6]["call_id"], "call-2");
    }

    #[test]
    fn deepseek_keeps_empty_assistant_beside_reasoning_for_tool_only_turns() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let mut request = tool_followup(true, "search the web");
        request.messages[1].tool_calls.push(StoredToolCall {
            id: "call-2".into(),
            name: "web_search".into(),
            arguments: "{\"q\":\"pizza\"}".into(),
        });
        request.messages.push(ChatMessage {
            role: "tool".into(),
            content: "second".into(),
            tool_call_id: Some("call-2".into()),
            ..Default::default()
        });
        let body = responses_body_for(&spec, &request);
        let input = body["input"].as_array().expect("input");
        assert_eq!(input[1]["type"], "reasoning");
        assert_eq!(input[1]["content"][0]["text"], "search the web");
        assert!(input[1].get("summary").is_none());
        assert_eq!(input[2]["role"], "assistant");
        assert_eq!(input[2]["content"], "");
        assert_eq!(input[3]["type"], "function_call");
        assert_eq!(input[3]["call_id"], "call-1");
        assert_eq!(input[4]["type"], "function_call");
        assert_eq!(input[4]["call_id"], "call-2");
        assert_eq!(input[5]["type"], "function_call_output");
        assert_eq!(input[5]["call_id"], "call-1");
        assert_eq!(input[6]["type"], "function_call_output");
        assert_eq!(input[6]["call_id"], "call-2");
    }

    #[test]
    fn deepseek_pairs_outputs_per_assistant_turn_not_globally() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let body = responses_body_for(
            &spec,
            &StepRequest {
                messages: vec![
                    ChatMessage {
                        role: "user".into(),
                        content: "search pizza".into(),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "assistant".into(),
                        reasoning: "first look".into(),
                        tool_calls: vec![
                            StoredToolCall {
                                id: "call-a".into(),
                                name: "web_search".into(),
                                arguments: "{\"q\":\"a\"}".into(),
                            },
                            StoredToolCall {
                                id: "call-b".into(),
                                name: "web_search".into(),
                                arguments: "{\"q\":\"b\"}".into(),
                            },
                        ],
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "tool".into(),
                        content: "a".into(),
                        tool_call_id: Some("call-a".into()),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "tool".into(),
                        content: "b".into(),
                        tool_call_id: Some("call-b".into()),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "assistant".into(),
                        reasoning: "second look".into(),
                        tool_calls: vec![
                            StoredToolCall {
                                id: "call-c".into(),
                                name: "web_search".into(),
                                arguments: "{\"q\":\"c\"}".into(),
                            },
                            StoredToolCall {
                                id: "call-d".into(),
                                name: "web_search".into(),
                                arguments: "{\"q\":\"d\"}".into(),
                            },
                        ],
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "tool".into(),
                        content: "c".into(),
                        tool_call_id: Some("call-c".into()),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "tool".into(),
                        content: "d".into(),
                        tool_call_id: Some("call-d".into()),
                        ..Default::default()
                    },
                ],
                instructions: None,
                thinking: true,
                tools: vec![ToolSpec {
                    name: "web_search".into(),
                    description: "Search".into(),
                    parameters: json!({ "type": "object", "properties": {} }),
                }],
            },
        );
        let input = body["input"].as_array().expect("input");
        let types: Vec<&str> = input
            .iter()
            .map(|item| {
                item.get("type")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("role").and_then(Value::as_str))
                    .unwrap_or("")
            })
            .collect();
        assert_eq!(
            types,
            [
                "user",
                "reasoning",
                "assistant",
                "function_call",
                "function_call",
                "function_call_output",
                "function_call_output",
                "reasoning",
                "assistant",
                "function_call",
                "function_call",
                "function_call_output",
                "function_call_output",
            ]
        );
        assert_eq!(input[1]["content"][0]["text"], "first look");
        assert_eq!(input[3]["call_id"], "call-a");
        assert_eq!(input[4]["call_id"], "call-b");
        assert_eq!(input[5]["call_id"], "call-a");
        assert_eq!(input[6]["call_id"], "call-b");
        assert_eq!(input[7]["content"][0]["text"], "second look");
        assert_eq!(input[9]["call_id"], "call-c");
        assert_eq!(input[10]["call_id"], "call-d");
        assert_eq!(input[11]["call_id"], "call-c");
        assert_eq!(input[12]["call_id"], "call-d");
    }

    #[test]
    fn deepseek_replays_text_only_reasoning_before_assistant_when_tools_present() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let body = responses_body_for(
            &spec,
            &StepRequest {
                messages: vec![
                    ChatMessage {
                        role: "user".into(),
                        content: "hi".into(),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "assistant".into(),
                        content: "hello".into(),
                        reasoning: "be helpful".into(),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "user".into(),
                        content: "search water".into(),
                        ..Default::default()
                    },
                ],
                instructions: None,
                thinking: true,
                tools: vec![ToolSpec {
                    name: "search_agent_tools".into(),
                    description: "Search tools".into(),
                    parameters: json!({ "type": "object", "properties": {} }),
                }],
            },
        );
        let input = body["input"].as_array().expect("input");
        assert_eq!(input[1]["type"], "reasoning");
        assert_eq!(input[1]["content"][0]["text"], "be helpful");
        assert_eq!(input[2]["role"], "assistant");
        assert_eq!(input[2]["content"], "hello");
        assert_eq!(input[3]["role"], "user");
        assert_eq!(input[3]["content"], "search water");
    }

    #[test]
    fn deepseek_synthesizes_reasoning_for_every_assistant_when_tools_present() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let body = responses_body_for(
            &spec,
            &StepRequest {
                messages: vec![
                    ChatMessage {
                        role: "user".into(),
                        content: "hi".into(),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "assistant".into(),
                        content: "hello".into(),
                        ..Default::default()
                    },
                    ChatMessage {
                        role: "user".into(),
                        content: "try again".into(),
                        ..Default::default()
                    },
                ],
                instructions: None,
                thinking: true,
                tools: vec![ToolSpec {
                    name: "web_search".into(),
                    description: "Search".into(),
                    parameters: json!({ "type": "object", "properties": {} }),
                }],
            },
        );
        let input = body["input"].as_array().expect("input");
        assert_eq!(input[1]["type"], "reasoning");
        assert_eq!(input[1]["content"][0]["type"], "reasoning_text");
        assert_eq!(input[1]["content"][0]["text"], ".");
        assert_eq!(input[2]["role"], "assistant");
        assert_eq!(input[2]["content"], "hello");
        assert_eq!(input[3]["role"], "user");
    }

    #[test]
    fn unpaired_function_call_gets_synthetic_output() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let mut request = tool_followup(true, "check tools");
        request.messages.pop();
        let body = responses_body_for(&spec, &request);
        let input = body["input"].as_array().expect("input");
        assert_eq!(input[2]["role"], "assistant");
        assert_eq!(input[3]["type"], "function_call");
        assert_eq!(input[4]["type"], "function_call_output");
        assert_eq!(input[4]["call_id"], "call-1");
        assert_eq!(input[4]["output"], "Tool result was not available.");
    }

    #[test]
    fn deepseek_responses_placeholder_when_thinking_has_no_text() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-responses"))
            .expect("model")
            .clone();
        let body = responses_body_for(&spec, &tool_followup(true, ""));
        let input = body["input"].as_array().expect("input");
        assert_eq!(input[1]["type"], "reasoning");
        assert_eq!(input[1]["content"][0]["text"], ".");
        assert_eq!(input[2]["role"], "assistant");
        assert_eq!(input[2]["content"], "");
        assert_eq!(input[3]["type"], "function_call");
    }

    #[test]
    fn openai_responses_do_not_replay_plaintext_reasoning() {
        let body = responses_body_for(&openai_spec(), &tool_followup(true, "check tools"));
        let input = body["input"].as_array().expect("input");
        assert!(input
            .iter()
            .all(|item| item.get("type").and_then(Value::as_str) != Some("reasoning")));
        assert_eq!(input[1]["type"], "function_call");
        assert_eq!(input[2]["type"], "function_call_output");
    }

    #[test]
    fn deepseek_anthropic_replays_thinking_block() {
        let spec = Catalog::builtin("k")
            .resolve(Some("deepseek-v4-flash-anthropic"))
            .expect("model")
            .clone();
        let body = anthropic_body_for(&spec, &tool_followup(true, "check tools"));
        let content = body["messages"][1]["content"].as_array().expect("content");
        assert_eq!(content[0]["type"], "thinking");
        assert_eq!(content[0]["thinking"], "check tools");
        assert_eq!(content[1]["type"], "tool_use");
        assert!(content[0].get("signature").is_none());
    }

    #[test]
    fn official_anthropic_does_not_inject_unsigned_thinking() {
        let body = anthropic_body_for(&claude_spec(), &tool_followup(true, "check tools"));
        let content = body["messages"][1]["content"].as_array().expect("content");
        assert!(content
            .iter()
            .all(|item| item.get("type").and_then(Value::as_str) != Some("thinking")));
        assert_eq!(content[0]["type"], "tool_use");
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
    fn parses_responses_reasoning_delta_object() {
        let body = "\
event: response.reasoning_text.delta\n\
data: {\"type\":\"response.reasoning_text.delta\",\"delta\":{\"text\":\"plan\"}}\n\
\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\
\n";
        let chunks = parse_provider_sse(Protocol::Responses, body);
        assert!(matches!(&chunks[0], LlmChunk::ReasoningDelta(text) if text == "plan"));
    }

    #[test]
    fn parses_responses_reasoning_from_done_when_deltas_omitted() {
        let body = "\
event: response.reasoning_text.done\n\
data: {\"type\":\"response.reasoning_text.done\",\"text\":\"full plan\"}\n\
\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\"}}\n\
\n";
        let chunks = parse_provider_sse(Protocol::Responses, body);
        assert!(matches!(&chunks[0], LlmChunk::ReasoningDelta(text) if text == "full plan"));
    }

    #[test]
    fn parses_responses_function_call_and_keeps_tool_calls_finish_reason() {
        let body = "\
event: response.output_item.done\n\
data: {\"type\":\"response.output_item.done\",\"item\":{\"type\":\"function_call\",\"call_id\":\"call-1\",\"name\":\"create_post\",\"arguments\":\"{\\\"title\\\":\\\"Тестовый пост\\\"}\"}}\n\
\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":12,\"output_tokens\":8}}}\n\
\n";
        let chunks = parse_provider_sse(Protocol::Responses, body);
        assert!(matches!(
            &chunks[0],
            LlmChunk::ToolCall { id, name, input }
                if id == "call-1" && name == "create_post" && input["title"] == "Тестовый пост"
        ));
        assert!(
            matches!(&chunks[2], LlmChunk::End { finish_reason } if finish_reason == "tool_calls")
        );
    }

    #[test]
    fn parses_responses_function_call_from_completed_output_when_stream_omits_item() {
        let body = "\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":4,\"output_tokens\":6},\"output\":[{\"type\":\"function_call\",\"call_id\":\"call-9\",\"name\":\"create_post\",\"arguments\":\"{\\\"body\\\":\\\"hi\\\"}\"}]}}\n\
\n";
        let chunks = parse_provider_sse(Protocol::Responses, body);
        assert!(matches!(
            &chunks[0],
            LlmChunk::ToolCall { id, name, .. } if id == "call-9" && name == "create_post"
        ));
        assert!(
            matches!(&chunks[2], LlmChunk::End { finish_reason } if finish_reason == "tool_calls")
        );
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

    #[tokio::test]
    async fn empty_api_key_fails_before_calling_the_provider() {
        let llm = HttpClient::from_timeout(1);
        let model = ModelSpec {
            id: "empty-key".into(),
            label: "Empty".into(),
            protocol: Protocol::Responses,
            api_model: "empty-key".into(),
            url: "http://127.0.0.1:1".into(),
            key: String::new(),
            aliases: Vec::new(),
        };
        let mut stream = llm.stream_step(&model, user("hi"));
        let first = stream.next().await.expect("chunk");
        match first {
            Err(LlmError::Other(message)) => {
                assert!(message.contains("Missing API key"));
                assert!(message.contains("DEEPSEEK_API_KEY"));
            }
            other => panic!("unexpected chunk: {other:?}"),
        }
    }
}
