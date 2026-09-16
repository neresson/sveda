use std::pin::Pin;
use std::sync::Arc;

use async_stream::stream;
use chrono::Utc;
use futures_util::{Stream, StreamExt};
use uuid::Uuid;
use veda_llm::{
    stream_with_failover, Catalog, ChatMessage, LlmChunk, LlmClient, LlmError, StepRequest,
    StoredToolCall, ToolSpec, UnknownModelError,
};
use veda_protocol::{StreamEvent, StreamUsage, ToolTarget};

mod compact;
mod history;
mod title;
mod tools;

pub use compact::{
    generate_summary, inject_summary, should_compact, summary_source, tail_messages,
};
pub use history::{
    chat_to_json, conversation_json, display_messages, json_to_chat, preview_from,
    split_prompt_and_history,
};
pub use title::{
    derive_provisional_title, generate_title, is_placeholder_title, sanitize_generated_title,
};
pub use tools::{
    MemoryBackend, ToolRuntime, READ_CODE_TOOL_NAME, SEARCH_CODE_TOOL_NAME, SEARCH_TOOL_NAME,
    SPAWN_TOOL_NAME,
};

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_steps: u32,
    pub context_max_tokens: u64,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 30,
            context_max_tokens: 128_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentRequest {
    pub prompt: String,
    pub history: Vec<ChatMessage>,
    pub chat_id: Option<String>,
    pub model: Option<String>,
    pub thinking: bool,
    pub client_tools: Vec<String>,
    pub client_tool_specs: Vec<ToolSpec>,
    pub instructions: Option<String>,
}

pub fn run(
    llm: Arc<dyn LlmClient>,
    catalog: Catalog,
    config: AgentConfig,
    request: AgentRequest,
) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, UnknownModelError> {
    run_with_tools(llm, catalog, config, request, ToolRuntime::default())
}

pub fn run_with_tools(
    llm: Arc<dyn LlmClient>,
    catalog: Catalog,
    config: AgentConfig,
    request: AgentRequest,
    runtime: ToolRuntime,
) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, UnknownModelError> {
    let primary = catalog.resolve(request.model.as_deref())?.clone();
    let chain = catalog.stream_chain(&primary);
    Ok(Box::pin(run_loop(llm, chain, config, request, runtime)))
}

fn run_loop(
    llm: Arc<dyn LlmClient>,
    chain: Vec<veda_llm::ModelSpec>,
    config: AgentConfig,
    request: AgentRequest,
    runtime: ToolRuntime,
) -> impl Stream<Item = StreamEvent> + Send {
    stream! {
        let chat_id = request.chat_id.clone();
        let message_id = Some(format!("msg_{}", Uuid::new_v4()));
        let timestamp = Some(Utc::now());
        let stamp = |event: StreamEvent| attach(event, chat_id.clone(), message_id.clone(), timestamp);
        let mut messages = request.history.clone();
        let prompt = request.prompt.trim();
        if !prompt.is_empty() {
            messages.push(ChatMessage {
                role: "user".into(),
                content: prompt.to_string(),
                ..Default::default()
            });
        }

        let mut started = false;
        let mut accumulated_prompt = 0u64;
        let mut accumulated_completion = 0u64;
        let mut final_reason: Option<String> = None;
        let mut stopped_for_frontend = false;
        let max_steps = config.max_steps.max(1);

        for step in 0..max_steps {
            let is_final = step + 1 >= max_steps;
            let mut advertised = runtime.advertised();
            for spec in &request.client_tool_specs {
                if !advertised.iter().any(|item| item.name == spec.name) {
                    advertised.push(spec.clone());
                }
            }
            let step_request = StepRequest {
                messages: messages.clone(),
                instructions: request.instructions.clone(),
                thinking: request.thinking,
                tools: advertised,
            };
            let mut llm_stream = stream_with_failover(llm.clone(), chain.clone(), step_request);
            let mut step_prompt = 0u64;
            let mut step_completion = 0u64;
            let mut finish_reason = String::from("stop");
            let mut tool_calls: Vec<(String, String, serde_json::Value)> = Vec::new();
            let mut assistant_text = String::new();
            let mut assistant_reasoning = String::new();
            let mut had_result = false;

            while let Some(item) = llm_stream.next().await {
                match item {
                    Ok(LlmChunk::TextDelta(delta)) => {
                        if !started {
                            yield stamp(StreamEvent::MessageStart {
                                chat_id: None,
                                message_id: None,
                                timestamp: None,
                            });
                            started = true;
                        }
                        assistant_text.push_str(&delta);
                        yield stamp(StreamEvent::TextDelta {
                            delta,
                            chat_id: None,
                            message_id: None,
                            timestamp: None,
                        });
                    }
                    Ok(LlmChunk::ReasoningDelta(delta)) => {
                        if !started {
                            yield stamp(StreamEvent::MessageStart {
                                chat_id: None,
                                message_id: None,
                                timestamp: None,
                            });
                            started = true;
                        }
                        assistant_reasoning.push_str(&delta);
                        yield stamp(StreamEvent::ReasoningDelta {
                            delta,
                            chat_id: None,
                            message_id: None,
                            timestamp: None,
                        });
                    }
                    Ok(LlmChunk::ToolCall { id, name, input }) => {
                        if !started {
                            yield stamp(StreamEvent::MessageStart {
                                chat_id: None,
                                message_id: None,
                                timestamp: None,
                            });
                            started = true;
                        }
                        let target = if request
                            .client_tools
                            .iter()
                            .any(|tool| tool == &name)
                        {
                            ToolTarget::Frontend
                        } else {
                            ToolTarget::Backend
                        };
                        tool_calls.push((id.clone(), name.clone(), input.clone()));
                        yield stamp(StreamEvent::ToolCall {
                            tool_call_id: id,
                            tool_name: name,
                            target,
                            input,
                            chat_id: None,
                            message_id: None,
                            timestamp: None,
                        });
                    }
                    Ok(LlmChunk::Usage {
                        prompt_tokens,
                        completion_tokens,
                    }) => {
                        step_prompt = prompt_tokens;
                        step_completion = completion_tokens;
                        had_result = true;
                    }
                    Ok(LlmChunk::End { finish_reason: reason }) => {
                        finish_reason = reason;
                        had_result = true;
                    }
                    Err(error) => {
                        yield stamp(StreamEvent::Error {
                            code: error_code(&error),
                            message: error.to_string(),
                            chat_id: None,
                            message_id: None,
                            timestamp: None,
                        });
                    }
                }
            }

            if had_result {
                accumulated_prompt += step_prompt;
                accumulated_completion += step_completion;
                final_reason = Some(finish_reason.clone());
                yield stamp(StreamEvent::ContextUsage {
                    used_tokens: step_prompt + step_completion,
                    max_tokens: config.context_max_tokens.max(1),
                    percent: usage_percent(
                        step_prompt + step_completion,
                        config.context_max_tokens.max(1),
                    ),
                    chat_id: None,
                    message_id: None,
                    timestamp: None,
                });
            }

            if finish_reason == "continue" {
                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: assistant_text,
                    reasoning: assistant_reasoning,
                    ..Default::default()
                });
                continue;
            }

            if finish_reason == "tool_calls" {
                let has_frontend = tool_calls.iter().any(|(_, name, _)| {
                    request.client_tools.iter().any(|tool| tool == name)
                });
                if has_frontend {
                    stopped_for_frontend = true;
                    break;
                }
                if is_final {
                    break;
                }
                let executable: Vec<(String, String, serde_json::Value)> = tool_calls
                    .iter()
                    .filter(|(_, name, _)| runtime.known(name))
                    .cloned()
                    .collect();
                if executable.is_empty() {
                    break;
                }
                messages.push(ChatMessage {
                    role: "assistant".into(),
                    content: assistant_text,
                    reasoning: assistant_reasoning,
                    tool_calls: executable
                        .iter()
                        .map(|(id, name, input)| StoredToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: input.to_string(),
                        })
                        .collect(),
                    ..Default::default()
                });
                for (id, name, input) in executable {
                    let outcome = runtime.execute(&name, input).await;
                    for event in outcome.events {
                        yield stamp(event);
                    }
                    let output = tools::normalize_output(outcome.output);
                    let links = tools::resource_links(&output);
                    yield stamp(StreamEvent::ToolResult {
                        tool_call_id: id.clone(),
                        tool_name: name,
                        output: output.clone(),
                        render_hint: links.as_ref().map(|_| "resource_links".to_string()),
                        render_data: links,
                        chat_id: None,
                        message_id: None,
                        timestamp: None,
                    });
                    messages.push(ChatMessage {
                        role: "tool".into(),
                        content: output.to_string(),
                        tool_call_id: Some(id),
                        ..Default::default()
                    });
                }
                continue;
            }

            break;
        }

        if final_reason.as_deref() == Some("tool_calls") && !stopped_for_frontend {
            yield stamp(StreamEvent::MaxSteps {
                max_steps,
                can_continue: true,
                chat_id: None,
                message_id: None,
                timestamp: None,
            });
            final_reason = Some("stop".into());
        }

        if started {
            yield stamp(StreamEvent::MessageEnd {
                finish_reason: Some(final_reason.unwrap_or_else(|| "error".into())),
                usage: Some(StreamUsage {
                    prompt_tokens: Some(accumulated_prompt),
                    completion_tokens: Some(accumulated_completion),
                    total_tokens: Some(accumulated_prompt + accumulated_completion),
                }),
                chat_id: None,
                message_id: None,
                timestamp: None,
            });
        }
    }
}

fn attach(
    mut event: StreamEvent,
    chat_id: Option<String>,
    message_id: Option<String>,
    timestamp: Option<chrono::DateTime<Utc>>,
) -> StreamEvent {
    match &mut event {
        StreamEvent::MessageStart {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
        }
        | StreamEvent::TextDelta {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::ReasoningDelta {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::ToolCall {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::ToolResult {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::ToolProgress {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::ContextUsage {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::MaxSteps {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::MessageEnd {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::Error {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        }
        | StreamEvent::ChatTitle {
            chat_id: slot_chat,
            message_id: slot_message,
            timestamp: slot_ts,
            ..
        } => {
            *slot_chat = chat_id;
            *slot_message = message_id;
            *slot_ts = timestamp;
        }
    }
    event
}

pub fn usage_percent(used: u64, max: u64) -> f64 {
    let max = max.max(1) as f64;
    let percent = (used as f64 / max * 100.0).min(100.0);
    (percent * 10.0).round() / 10.0
}

fn error_code(error: &LlmError) -> String {
    match error {
        LlmError::RateLimited => "rate_limited".into(),
        LlmError::Overloaded => "overloaded".into(),
        LlmError::InsufficientCredits => "insufficient_credits".into(),
        LlmError::Other(_) => "llm_error".into(),
    }
}

pub fn extract_prompt(prompt: Option<&str>, messages: &[serde_json::Value]) -> String {
    if let Some(prompt) = prompt.map(str::trim).filter(|value| !value.is_empty()) {
        return prompt.to_string();
    }
    let mut items = messages.to_vec();
    while let Some(last) = items.last() {
        if last.get("role").and_then(|role| role.as_str()) != Some("assistant") {
            break;
        }
        if history::message_text(last).is_empty() {
            items.pop();
            continue;
        }
        break;
    }
    items
        .last()
        .filter(|message| message.get("role").and_then(|role| role.as_str()) == Some("user"))
        .map(history::message_text)
        .unwrap_or_default()
}

pub fn thinking_enabled(options: Option<&serde_json::Value>) -> bool {
    options
        .and_then(|value| value.get("thinking"))
        .and_then(|value| value.as_bool())
        != Some(false)
}

pub fn client_tool_names(client_tools: Option<&Vec<serde_json::Value>>) -> Vec<String> {
    client_tool_specs(client_tools)
        .into_iter()
        .map(|tool| tool.name)
        .collect()
}

pub fn client_tool_specs(client_tools: Option<&Vec<serde_json::Value>>) -> Vec<ToolSpec> {
    client_tools
        .map(|tools| {
            tools
                .iter()
                .filter_map(|tool| {
                    let name = tool.get("name").and_then(|name| name.as_str())?;
                    Some(ToolSpec {
                        name: name.to_string(),
                        description: tool
                            .get("description")
                            .and_then(|value| value.as_str())
                            .unwrap_or("")
                            .to_string(),
                        parameters: tool.get("parameters").cloned().unwrap_or_else(
                            || serde_json::json!({ "type": "object", "properties": {} }),
                        ),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;
    use serde_json::json;
    use veda_llm::{LlmClient, ScriptedClient};

    fn request() -> AgentRequest {
        AgentRequest {
            prompt: "Say hello".into(),
            history: Vec::new(),
            chat_id: Some("chat-1".into()),
            model: None,
            thinking: true,
            client_tools: Vec::new(),
            client_tool_specs: Vec::new(),
            instructions: None,
        }
    }

    async fn collect_async(
        llm: ScriptedClient,
        request: AgentRequest,
        config: AgentConfig,
    ) -> Vec<StreamEvent> {
        let catalog = Catalog::builtin("");
        let mut stream = run(Arc::new(llm), catalog, config, request).expect("catalog");
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        events
    }

    fn types_of(events: &[StreamEvent]) -> Vec<&'static str> {
        events.iter().map(StreamEvent::event_type).collect()
    }

    #[tokio::test]
    async fn text_only_turn_emits_start_delta_usage_end() {
        let events = collect_async(
            ScriptedClient::hello_world(),
            request(),
            AgentConfig::default(),
        )
        .await;
        let types = types_of(&events);
        assert_eq!(
            types,
            vec![
                "message.start",
                "text.delta",
                "context.usage",
                "message.end"
            ]
        );
        match &events[3] {
            StreamEvent::MessageEnd {
                finish_reason,
                usage,
                ..
            } => {
                assert_eq!(finish_reason.as_deref(), Some("stop"));
                assert_eq!(usage.as_ref().unwrap().total_tokens, Some(3));
            }
            other => panic!("expected message.end, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn reasoning_deltas_are_forwarded() {
        let llm = ScriptedClient::queue(vec![vec![
            Ok(LlmChunk::ReasoningDelta("plan".into())),
            Ok(LlmChunk::TextDelta("Hi".into())),
            Ok(LlmChunk::Usage {
                prompt_tokens: 4,
                completion_tokens: 2,
            }),
            Ok(LlmChunk::End {
                finish_reason: "stop".into(),
            }),
        ]]);
        let events = collect_async(llm, request(), AgentConfig::default()).await;
        assert!(types_of(&events).contains(&"reasoning.delta"));
    }

    #[tokio::test]
    async fn context_usage_percent_uses_config_cap() {
        let llm = ScriptedClient::queue(vec![vec![
            Ok(LlmChunk::TextDelta("Hi".into())),
            Ok(LlmChunk::Usage {
                prompt_tokens: 100,
                completion_tokens: 50,
            }),
            Ok(LlmChunk::End {
                finish_reason: "stop".into(),
            }),
        ]]);
        let events = collect_async(
            llm,
            request(),
            AgentConfig {
                max_steps: 5,
                context_max_tokens: 1000,
            },
        )
        .await;
        let usage = events
            .iter()
            .find_map(|event| match event {
                StreamEvent::ContextUsage {
                    used_tokens,
                    max_tokens,
                    percent,
                    ..
                } => Some((*used_tokens, *max_tokens, *percent)),
                _ => None,
            })
            .unwrap();
        assert_eq!(usage, (150, 1000, 15.0));
    }

    #[tokio::test]
    async fn backend_tool_call_emits_max_steps_then_stop() {
        let llm = ScriptedClient::queue(vec![vec![
            Ok(LlmChunk::ToolCall {
                id: "call-1".into(),
                name: "unknown_backend_tool".into(),
                input: json!({"x": 1}),
            }),
            Ok(LlmChunk::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
            Ok(LlmChunk::End {
                finish_reason: "tool_calls".into(),
            }),
        ]]);
        let events = collect_async(
            llm,
            request(),
            AgentConfig {
                max_steps: 1,
                context_max_tokens: 128000,
            },
        )
        .await;
        assert!(types_of(&events).contains(&"max_steps"));
        match events.last() {
            Some(StreamEvent::MessageEnd { finish_reason, .. }) => {
                assert_eq!(finish_reason.as_deref(), Some("stop"));
            }
            other => panic!("expected message.end, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn frontend_tool_stops_without_max_steps() {
        let llm = ScriptedClient::queue(vec![vec![
            Ok(LlmChunk::ToolCall {
                id: "call-1".into(),
                name: "confirm_course_creation".into(),
                input: json!({"title": "Math"}),
            }),
            Ok(LlmChunk::Usage {
                prompt_tokens: 10,
                completion_tokens: 5,
            }),
            Ok(LlmChunk::End {
                finish_reason: "tool_calls".into(),
            }),
        ]]);
        let mut request = request();
        request.client_tools = vec!["confirm_course_creation".into()];
        let events = collect_async(llm, request, AgentConfig::default()).await;
        assert!(!types_of(&events).contains(&"max_steps"));
        match events.last() {
            Some(StreamEvent::MessageEnd { finish_reason, .. }) => {
                assert_eq!(finish_reason.as_deref(), Some("tool_calls"));
            }
            other => panic!("expected message.end, got {other:?}"),
        }
        match events
            .iter()
            .find(|event| event.event_type() == "tool.call")
        {
            Some(StreamEvent::ToolCall { target, .. }) => {
                assert_eq!(*target, ToolTarget::Frontend);
            }
            other => panic!("expected tool.call, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn continue_finish_reason_runs_another_step() {
        let llm = ScriptedClient::queue(vec![
            vec![
                Ok(LlmChunk::TextDelta("part1".into())),
                Ok(LlmChunk::Usage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                }),
                Ok(LlmChunk::End {
                    finish_reason: "continue".into(),
                }),
            ],
            vec![
                Ok(LlmChunk::TextDelta("part2".into())),
                Ok(LlmChunk::Usage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                }),
                Ok(LlmChunk::End {
                    finish_reason: "stop".into(),
                }),
            ],
        ]);
        let events = collect_async(llm, request(), AgentConfig::default()).await;
        let deltas: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(deltas, vec!["part1", "part2"]);
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type() == "context.usage")
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn unknown_model_is_an_error() {
        let catalog = Catalog::builtin("");
        let result = run(
            Arc::new(ScriptedClient::hello_world()),
            catalog,
            AgentConfig::default(),
            AgentRequest {
                model: Some("no-such-model".into()),
                ..request()
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn omitted_thinking_stays_on() {
        assert!(thinking_enabled(None));
        assert!(thinking_enabled(Some(&json!({}))));
        assert!(!thinking_enabled(Some(&json!({ "thinking": false }))));
        assert!(thinking_enabled(Some(&json!({ "thinking": true }))));
    }

    #[test]
    fn extracts_prompt_from_trailing_empty_assistant() {
        let messages = vec![
            json!({"id": "m1", "role": "user", "content": "Say hello"}),
            json!({"id": "m2", "role": "assistant", "parts": []}),
        ];
        assert_eq!(extract_prompt(None, &messages), "Say hello");
    }

    async fn collect_with(
        llm: ScriptedClient,
        request: AgentRequest,
        config: AgentConfig,
        runtime: ToolRuntime,
    ) -> Vec<StreamEvent> {
        let catalog = Catalog::builtin("");
        let mut stream =
            run_with_tools(Arc::new(llm), catalog, config, request, runtime).expect("catalog");
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            events.push(event);
        }
        events
    }

    fn calendar_tool() -> veda_mcp::McpTool {
        veda_mcp::McpTool {
            name: "get_calendar_events".into(),
            description: "Get calendar events".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
            domain: "calendar".into(),
            mode: "read".into(),
        }
    }

    #[tokio::test]
    async fn backend_mcp_tool_executes_and_loop_continues() {
        let llm = ScriptedClient::queue(vec![
            vec![
                Ok(LlmChunk::ToolCall {
                    id: "call-1".into(),
                    name: "get_calendar_events".into(),
                    input: json!({ "limit": 1 }),
                }),
                Ok(LlmChunk::Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                }),
                Ok(LlmChunk::End {
                    finish_reason: "tool_calls".into(),
                }),
            ],
            vec![
                Ok(LlmChunk::TextDelta("Done".into())),
                Ok(LlmChunk::Usage {
                    prompt_tokens: 8,
                    completion_tokens: 4,
                }),
                Ok(LlmChunk::End {
                    finish_reason: "stop".into(),
                }),
            ],
        ]);
        let runtime = ToolRuntime::memory(
            MemoryBackend::new(
                [(
                    "get_calendar_events".into(),
                    json!({ "success": true, "data": { "events": [] } }),
                )]
                .into_iter()
                .collect(),
            ),
            vec![calendar_tool()],
        )
        .with_defer(false, 14);
        let events = collect_with(llm, request(), AgentConfig::default(), runtime).await;
        assert!(types_of(&events).contains(&"tool.result"));
        assert!(!types_of(&events).contains(&"max_steps"));
        match events.last() {
            Some(StreamEvent::MessageEnd { finish_reason, .. }) => {
                assert_eq!(finish_reason.as_deref(), Some("stop"));
            }
            other => panic!("expected message.end, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn thinking_tool_loop_keeps_reasoning_on_assistant() {
        let llm = Arc::new(ScriptedClient::queue(vec![
            vec![
                Ok(LlmChunk::ReasoningDelta("check tools".into())),
                Ok(LlmChunk::ToolCall {
                    id: "call-1".into(),
                    name: "get_calendar_events".into(),
                    input: json!({ "limit": 1 }),
                }),
                Ok(LlmChunk::Usage {
                    prompt_tokens: 10,
                    completion_tokens: 5,
                }),
                Ok(LlmChunk::End {
                    finish_reason: "tool_calls".into(),
                }),
            ],
            vec![
                Ok(LlmChunk::TextDelta("Done".into())),
                Ok(LlmChunk::Usage {
                    prompt_tokens: 8,
                    completion_tokens: 4,
                }),
                Ok(LlmChunk::End {
                    finish_reason: "stop".into(),
                }),
            ],
        ]));
        let runtime = ToolRuntime::memory(
            MemoryBackend::new(
                [(
                    "get_calendar_events".into(),
                    json!({ "success": true, "data": { "events": [] } }),
                )]
                .into_iter()
                .collect(),
            ),
            vec![calendar_tool()],
        )
        .with_defer(false, 14);
        let catalog = Catalog::builtin("");
        let mut stream = run_with_tools(
            llm.clone() as Arc<dyn LlmClient>,
            catalog,
            AgentConfig::default(),
            request(),
            runtime,
        )
        .expect("catalog");
        while stream.next().await.is_some() {}
        let steps = llm.last_messages.lock().expect("messages");
        assert_eq!(steps.len(), 2);
        let assistant = steps[1]
            .iter()
            .find(|message| message.role == "assistant")
            .expect("assistant");
        assert_eq!(assistant.reasoning, "check tools");
        assert_eq!(assistant.tool_calls[0].name, "get_calendar_events");
    }

    #[test]
    fn chat_json_roundtrips_reasoning() {
        let message = ChatMessage {
            role: "assistant".into(),
            content: "hi".into(),
            reasoning: "plan".into(),
            ..Default::default()
        };
        let value = chat_to_json(&message);
        let restored = json_to_chat(&value).expect("chat");
        assert_eq!(restored.reasoning, "plan");
        assert_eq!(restored.content, "hi");
    }

    #[tokio::test]
    async fn spawn_tasks_emits_progress_and_runs_host_tools() {
        let llm = ScriptedClient::queue(vec![
            vec![
                Ok(LlmChunk::ToolCall {
                    id: "call-1".into(),
                    name: "spawn_tasks".into(),
                    input: json!({
                        "tasks": [
                            { "type": "get_calendar_events", "input": { "limit": 1 }, "label": "Calendar" }
                        ]
                    }),
                }),
                Ok(LlmChunk::Usage {
                    prompt_tokens: 4,
                    completion_tokens: 2,
                }),
                Ok(LlmChunk::End {
                    finish_reason: "tool_calls".into(),
                }),
            ],
            vec![
                Ok(LlmChunk::TextDelta("ok".into())),
                Ok(LlmChunk::Usage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                }),
                Ok(LlmChunk::End {
                    finish_reason: "stop".into(),
                }),
            ],
        ]);
        let runtime = ToolRuntime::memory(
            MemoryBackend::new(
                [(
                    "get_calendar_events".into(),
                    json!({ "success": true, "summary": "2 events", "data": { "events": [] } }),
                )]
                .into_iter()
                .collect(),
            ),
            vec![calendar_tool()],
        )
        .with_defer(false, 14);
        let events = collect_with(llm, request(), AgentConfig::default(), runtime).await;
        assert!(types_of(&events).contains(&"tool.progress"));
        assert!(types_of(&events).contains(&"tool.result"));
    }

    #[tokio::test]
    async fn deferral_hides_mcp_tools_until_search() {
        let llm = Arc::new(ScriptedClient::hello_world());
        let tools = (0..14)
            .map(|index| veda_mcp::McpTool {
                name: format!("tool_{index}"),
                description: if index == 3 {
                    "create invoice billing".into()
                } else {
                    "other".into()
                },
                input_schema: json!({ "type": "object" }),
                domain: "billing".into(),
                mode: "read".into(),
            })
            .collect();
        let runtime = ToolRuntime::memory(MemoryBackend::new(Default::default()), tools);
        let catalog = Catalog::builtin("");
        let mut stream = run_with_tools(
            llm.clone() as Arc<dyn LlmClient>,
            catalog,
            AgentConfig::default(),
            request(),
            runtime,
        )
        .expect("catalog");
        while stream.next().await.is_some() {}
        let advertised = llm.last_tools.lock().expect("tools");
        let first = advertised.first().cloned().unwrap_or_default();
        assert!(first.contains(&"spawn_tasks".to_string()));
        assert!(first.contains(&"search_agent_tools".to_string()));
        assert!(!first.contains(&"tool_3".to_string()));
    }

    #[test]
    fn placeholder_titles_match_php() {
        assert!(is_placeholder_title(None));
        assert!(is_placeholder_title(Some("")));
        assert!(is_placeholder_title(Some("New Chat")));
        assert!(is_placeholder_title(Some("Новый чат 2")));
        assert!(!is_placeholder_title(Some("Ocean tides")));
    }

    #[test]
    fn compaction_keeps_tail() {
        let messages = (0..5)
            .map(|index| ChatMessage {
                role: "user".into(),
                content: format!("m{index}"),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let tail = tail_messages(&messages, 2);
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].content, "m3");
        assert!(should_compact(40, true, 40));
        assert!(!should_compact(3, true, 40));
    }

    #[test]
    fn split_uses_last_user_as_prompt() {
        let (prompt, history) = split_prompt_and_history(
            None,
            &[
                json!({"role": "user", "content": "first"}),
                json!({"role": "assistant", "content": "ok"}),
                json!({"role": "user", "content": "second"}),
            ],
        );
        assert_eq!(prompt, "second");
        assert_eq!(history.len(), 2);
    }
}
