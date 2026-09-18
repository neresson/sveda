use std::convert::Infallible;
use std::pin::Pin;

use async_stream::stream;
use axum::response::IntoResponse;
use chrono::Utc;
use futures_util::{Stream, StreamExt};
use sveda_agent::{
    conversation_json, derive_provisional_title, display_messages, generate_summary,
    generate_title, inject_summary, is_placeholder_title, json_to_chat, preview_from,
    should_compact, split_prompt_and_history, tail_messages, AgentConfig, AgentRequest,
    ToolRuntime,
};
use sveda_llm::ChatMessage;
use sveda_mcp::{HostMcpClient, McpCallContext, McpCredentials};
use sveda_protocol::{StreamEvent, StreamRequest};
use sveda_store::Checkpoint;
use uuid::Uuid;

use crate::AppState;

pub async fn run_turn(
    state: AppState,
    visitor_id: String,
    request: StreamRequest,
) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, Box<axum::response::Response>> {
    let chat_id = request
        .chat_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("chat_{}", Uuid::new_v4()));
    let incoming = request.messages.clone().unwrap_or_default();
    let (prompt, mut history) = split_prompt_and_history(request.prompt.as_deref(), &incoming);
    let stored = state
        .store
        .record(&visitor_id, &chat_id)
        .await
        .ok()
        .flatten();
    if history.is_empty() {
        if let Some(record) = &stored {
            history = record
                .conversation_history
                .iter()
                .filter_map(json_to_chat)
                .collect();
        }
    }
    let runtime_config = state.runtime_config();
    let catalog = state.catalog();
    let full_history = history.clone();
    if stored
        .as_ref()
        .and_then(|record| record.summary.as_deref())
        .is_some()
    {
        history = tail_messages(&history, runtime_config.compaction_keep_tail);
    }
    let mut instructions = inject_summary(
        Some(runtime_config.system_prompt.clone()).filter(|value| !value.trim().is_empty()),
        stored.as_ref().and_then(|record| record.summary.as_deref()),
    );

    let mut runtime = ToolRuntime::default()
        .with_defer(runtime_config.defer_enabled, runtime_config.defer_min_pool);
    if let Some(creds) = state.host_mcp(&visitor_id) {
        let client = HostMcpClient::from_timeout(
            runtime_config.mcp_timeout,
            McpCredentials {
                url: creds.url,
                token: creds.token,
            },
            McpCallContext {
                page_context: request.context.clone(),
                chat_id: Some(chat_id.clone()),
            },
        );
        if let Ok(tools) = client.list_tools().await {
            let tool_names: Vec<String> = tools.iter().map(|tool| tool.name.clone()).collect();
            let host_instructions =
                sveda_agent::host_tools_instruction(client.instructions().as_deref(), &tool_names);
            instructions = sveda_agent::append_instruction(instructions, &host_instructions);
            runtime = ToolRuntime::mcp(std::sync::Arc::new(client), tools)
                .with_defer(runtime_config.defer_enabled, runtime_config.defer_min_pool);
        }
    }
    if let Some(index) = state.index.clone() {
        runtime = runtime.with_index(index);
    }

    let client_tool_specs = sveda_agent::client_tool_specs(request.client_tools.as_ref());
    let agent_request = AgentRequest {
        prompt: prompt.clone(),
        history,
        chat_id: Some(chat_id.clone()),
        model: request.model.clone(),
        thinking: sveda_agent::thinking_enabled(request.options.as_ref()),
        client_tools: client_tool_specs
            .iter()
            .map(|tool| tool.name.clone())
            .collect(),
        client_tool_specs,
        instructions,
    };

    let events = match sveda_agent::run_with_tools(
        state.llm.clone(),
        catalog.clone(),
        AgentConfig {
            max_steps: runtime_config.max_steps,
            context_max_tokens: runtime_config.context_max_tokens,
        },
        agent_request,
        runtime,
    ) {
        Ok(events) => events,
        Err(error) => {
            return Err(Box::new(
                (
                    axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                    axum::Json(serde_json::json!({ "message": error.to_string() })),
                )
                    .into_response(),
            ));
        }
    };

    Ok(Box::pin(finalize_turn(
        state,
        visitor_id,
        chat_id,
        prompt,
        incoming,
        full_history,
        events,
    )))
}

fn finalize_turn(
    state: AppState,
    visitor_id: String,
    chat_id: String,
    prompt: String,
    incoming: Vec<serde_json::Value>,
    full_history: Vec<ChatMessage>,
    events: Pin<Box<dyn Stream<Item = StreamEvent> + Send>>,
) -> impl Stream<Item = StreamEvent> + Send {
    stream! {
        let mut events = events;
        let mut assistant = String::new();
        let mut reasoning = String::new();
        let mut tokens = 0u64;
        let mut errored = false;
        while let Some(event) = events.next().await {
            match &event {
                StreamEvent::TextDelta { delta, .. } => assistant.push_str(delta),
                StreamEvent::ReasoningDelta { delta, .. } => reasoning.push_str(delta),
                StreamEvent::MessageEnd { usage, .. } => {
                    tokens = usage
                        .as_ref()
                        .and_then(|usage| usage.total_tokens)
                        .unwrap_or(0);
                }
                StreamEvent::Error { .. } => errored = true,
                _ => {}
            }
            yield event;
        }

        let runtime_config = state.runtime_config();
        let catalog = state.catalog();
        let existing = state
            .store
            .record(&visitor_id, &chat_id)
            .await
            .ok()
            .flatten();
        let mut title = existing
            .as_ref()
            .map(|record| record.title.clone())
            .unwrap_or_default();
        if !errored
            && runtime_config.title_generation_enabled
            && is_placeholder_title(Some(&title))
        {
            title = generate_title(state.llm.clone(), &catalog, &prompt).await;
            if !title.is_empty() {
                yield StreamEvent::ChatTitle {
                    title: title.clone(),
                    chat_id: Some(chat_id.clone()),
                    message_id: None,
                    timestamp: Some(Utc::now()),
                };
            }
        }
        if title.is_empty() {
            title = derive_provisional_title(&prompt);
        }

        let messages = display_messages(&incoming, &prompt, &assistant, "assistant_turn");
        let conversation = conversation_json(&full_history, &prompt, &assistant, &reasoning);
        let _ = state
            .store
            .checkpoint(Checkpoint {
                visitor_id: visitor_id.clone(),
                chat_id: chat_id.clone(),
                title,
                preview: preview_from(&messages, &prompt),
                messages,
                conversation_history: conversation.clone(),
                tokens_used: tokens,
            })
            .await;

        if should_compact(
            conversation.len(),
            runtime_config.compaction_enabled,
            runtime_config.compaction_min_messages,
        ) {
            let llm = state.llm.clone();
            let store = state.store.clone();
            tokio::spawn(async move {
                let chats: Vec<ChatMessage> =
                    conversation.iter().filter_map(json_to_chat).collect();
                let summary = generate_summary(llm, &catalog, &chats).await;
                if !summary.is_empty() {
                    let _ = store.set_summary(&visitor_id, &chat_id, summary).await;
                }
            });
        }
    }
}

pub fn sse_done() -> axum::response::sse::Event {
    axum::response::sse::Event::default().data("[DONE]")
}

pub fn sse_event(event: &StreamEvent) -> Result<axum::response::sse::Event, Infallible> {
    Ok(axum::response::sse::Event::default().data(serde_json::to_string(event).expect("event")))
}
