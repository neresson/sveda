use std::collections::HashSet;
use std::convert::Infallible;
use std::pin::Pin;

use async_stream::stream;
use axum::response::IntoResponse;
use chrono::Utc;
use futures_util::{Stream, StreamExt};
use sveda_agent::{
    conversation_json, denied_tool_output, derive_provisional_title, display_messages,
    generate_summary, generate_title, inject_summary, is_placeholder_title, json_to_chat,
    normalize_output, pending_tool_calls, preview_from, resource_links, should_compact,
    split_prompt_and_history, tail_messages, AgentConfig, AgentRequest, PendingToolCall,
    ToolRuntime,
};
use sveda_llm::{ChatMessage, StoredToolCall};
use sveda_mcp::{HostMcpClient, McpCallContext, McpCredentials};
use sveda_protocol::{StreamEvent, StreamRequest, ToolDecision};
use sveda_store::{Checkpoint, UsageEvent};
use uuid::Uuid;

use crate::admin_tools;
use crate::policy::{
    claims_restricted, filter_client_tools, filter_mcp_tools, resolve_effective,
    EffectiveCapabilities,
};
use crate::token::TokenClaims;
use crate::web;
use crate::AppState;

pub async fn run_turn(
    state: AppState,
    claims: TokenClaims,
    request: StreamRequest,
) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, Box<axum::response::Response>> {
    let visitor_id = claims.visitor_id.clone();
    let admin = claims.admin;
    let chat_id = request
        .chat_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("chat_{}", Uuid::new_v4()));
    let mut incoming = request.messages.clone().unwrap_or_default();
    let (mut prompt, mut history) = split_prompt_and_history(request.prompt.as_deref(), &incoming);
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
    let model_id = catalog
        .resolve(request.model.as_deref())
        .map(|model| model.id.clone())
        .unwrap_or_default();
    let mut full_history = history.clone();
    if stored
        .as_ref()
        .and_then(|record| record.summary.as_deref())
        .is_some()
    {
        history = tail_messages(&history, runtime_config.compaction_keep_tail);
    }
    let mut instructions = inject_summary(
        if admin {
            Some(admin_tools::instructions(request.context.as_ref()))
        } else {
            Some(runtime_config.system_prompt.clone()).filter(|value| !value.trim().is_empty())
        },
        stored.as_ref().and_then(|record| record.summary.as_deref()),
    );

    let restricted = !admin && claims_restricted(claims.policy.as_deref(), claims.grants.as_ref());
    let capabilities = if admin {
        EffectiveCapabilities::unrestricted()
    } else if restricted {
        let settings = state.settings.document();
        match resolve_effective(
            &settings.policies,
            claims.policy.as_deref(),
            claims.grants.as_ref(),
        ) {
            Some(Ok(caps)) => caps,
            Some(Err(message)) => {
                return Err(Box::new(
                    (
                        axum::http::StatusCode::FORBIDDEN,
                        axum::Json(serde_json::json!({ "message": message })),
                    )
                        .into_response(),
                ));
            }
            None => EffectiveCapabilities::unrestricted(),
        }
    } else {
        EffectiveCapabilities::unrestricted()
    };

    let mut runtime = ToolRuntime::default()
        .with_defer(runtime_config.defer_enabled, runtime_config.defer_min_pool);
    if admin {
        runtime = runtime.with_native(admin_tools::tools(), admin_tools::handler(state.clone()));
    } else {
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
                let tools = filter_mcp_tools(tools, &capabilities.mcp, restricted);
                let tool_names: Vec<String> = tools.iter().map(|tool| tool.name.clone()).collect();
                let host_instructions = sveda_agent::host_tools_instruction(
                    client.instructions().as_deref(),
                    &tool_names,
                );
                instructions = sveda_agent::append_instruction(instructions, &host_instructions);
                runtime = ToolRuntime::mcp(std::sync::Arc::new(client), tools)
                    .with_defer(runtime_config.defer_enabled, runtime_config.defer_min_pool);
            }
        }
        if !restricted || capabilities.code {
            let workspace = state.index.lock().expect("index").clone();
            let sources = state.code_index.ready_indexes();
            let uses_provider = workspace
                .as_ref()
                .map(|index| index.uses_provider())
                .unwrap_or(false)
                || sources.iter().any(|index| index.uses_provider());
            if let Some(index) = workspace {
                runtime = runtime.with_index(index);
            }
            if !sources.is_empty() {
                runtime = runtime.with_source_indexes(sources);
            }
            if uses_provider {
                if let Some(spec) = state.settings.document().active_embedding() {
                    if !spec.key.trim().is_empty() {
                        let http = state.embed_http.clone();
                        runtime = runtime.with_query_embedder(std::sync::Arc::new(
                            move |text: String| {
                                let http = http.clone();
                                let spec = spec.clone();
                                Box::pin(async move {
                                    http.embed(&spec, &[text])
                                        .await
                                        .map_err(|error| error.to_string())?
                                        .into_iter()
                                        .next()
                                        .ok_or_else(|| "empty embedding".to_string())
                                })
                            },
                        ));
                    }
                }
            }
        }
        if runtime_config.web_enabled && (!restricted || capabilities.web) {
            instructions = sveda_agent::append_instruction(instructions, &web::instructions());
            runtime = runtime.with_native(
                web::tools(),
                web::handler(web::WebConfig::from_runtime(&runtime_config)),
            );
        }
    }

    let filtered_client_tools = filter_client_tools(
        request.client_tools.as_ref(),
        &capabilities.client,
        restricted,
    );
    let client_tool_specs = sveda_agent::client_tool_specs(filtered_client_tools.as_ref());
    let tool_decisions = request.tool_decisions.clone();
    let mut decision_events = Vec::new();
    let mut skip_model = false;
    if !tool_decisions.is_empty() && stored.is_none() {
        return Err(confirmation_response("Chat history is unavailable."));
    }
    if let Some(record) = stored.as_ref() {
        let pending = confirming_pending(&runtime, &record.conversation_history);
        if !pending.is_empty() && tool_decisions.is_empty() {
            return Err(confirmation_response(
                "Tool confirmation is required before the conversation can continue.",
            ));
        }
        if !pending.is_empty() {
            history = record
                .conversation_history
                .iter()
                .filter_map(json_to_chat)
                .collect();
            full_history = history.clone();
            if record.summary.as_deref().is_some() {
                history = tail_messages(&history, runtime_config.compaction_keep_tail);
            }
            incoming = record.messages.clone();
            prompt = String::new();
            decision_events = resolve_tool_decisions(&runtime, &pending, &tool_decisions).await;
            let resolved: HashSet<String> = decision_events
                .iter()
                .filter_map(|event| match event {
                    StreamEvent::ToolResult { tool_call_id, .. } => Some(tool_call_id.clone()),
                    _ => None,
                })
                .collect();
            for event in &decision_events {
                if let StreamEvent::ToolResult {
                    tool_call_id,
                    output,
                    ..
                } = event
                {
                    history.push(ChatMessage {
                        role: "tool".into(),
                        content: output.to_string(),
                        tool_call_id: Some(tool_call_id.clone()),
                        ..Default::default()
                    });
                }
            }
            skip_model = pending.iter().any(|call| !resolved.contains(&call.id));
        }
    }

    let events: Pin<Box<dyn Stream<Item = StreamEvent> + Send>> = if skip_model {
        let chat_id = chat_id.clone();
        Box::pin(stream! {
            for event in decision_events {
                yield event;
            }
            yield StreamEvent::MessageEnd {
                finish_reason: Some("tool_calls".into()),
                usage: None,
                chat_id: Some(chat_id),
                message_id: None,
                timestamp: Some(Utc::now()),
            };
        })
    } else {
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
            confirming_client_tools: sveda_agent::confirming_client_tool_names(
                filtered_client_tools.as_ref(),
            ),
            client_tool_specs,
            instructions,
        };
        let agent_events = match sveda_agent::run_with_tools(
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
        Box::pin(stream! {
            for event in decision_events {
                yield event;
            }
            let mut agent_events = agent_events;
            while let Some(event) = agent_events.next().await {
                yield event;
            }
        })
    };

    Ok(Box::pin(finalize_turn(
        state,
        visitor_id,
        chat_id,
        model_id,
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
    model_id: String,
    prompt: String,
    incoming: Vec<serde_json::Value>,
    full_history: Vec<ChatMessage>,
    events: Pin<Box<dyn Stream<Item = StreamEvent> + Send>>,
) -> impl Stream<Item = StreamEvent> + Send {
    stream! {
        let mut events = events;
        let mut assistant = String::new();
        let mut reasoning = String::new();
        let mut tool_calls: Vec<StoredToolCall> = Vec::new();
        let mut confirming_ids: Vec<String> = Vec::new();
        let mut tool_results: Vec<(String, String, String)> = Vec::new();
        let mut tokens = 0u64;
        let mut prompt_tokens = 0u64;
        let mut completion_tokens = 0u64;
        let mut errored = false;
        while let Some(event) = events.next().await {
            match &event {
                StreamEvent::TextDelta { delta, .. } => assistant.push_str(delta),
                StreamEvent::ReasoningDelta { delta, .. } => reasoning.push_str(delta),
                StreamEvent::ToolCall {
                    tool_call_id,
                    tool_name,
                    input,
                    confirmation,
                    ..
                } => {
                    if confirmation.as_deref() == Some("required") {
                        confirming_ids.push(tool_call_id.clone());
                    }
                    tool_calls.push(StoredToolCall {
                        id: tool_call_id.clone(),
                        name: tool_name.clone(),
                        arguments: input.to_string(),
                    });
                }
                StreamEvent::ToolResult {
                    tool_call_id,
                    tool_name,
                    output,
                    ..
                } => tool_results.push((
                    tool_call_id.clone(),
                    tool_name.clone(),
                    output.to_string(),
                )),
                StreamEvent::MessageEnd { usage, .. } => {
                    prompt_tokens = usage
                        .as_ref()
                        .and_then(|usage| usage.prompt_tokens)
                        .unwrap_or(0);
                    completion_tokens = usage
                        .as_ref()
                        .and_then(|usage| usage.completion_tokens)
                        .unwrap_or(0);
                    tokens = usage
                        .as_ref()
                        .and_then(|usage| usage.total_tokens)
                        .unwrap_or(prompt_tokens.saturating_add(completion_tokens));
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
        let stored_title = existing
            .as_ref()
            .map(|record| record.title.clone())
            .unwrap_or_default();
        let should_generate_title = !errored
            && runtime_config.title_generation_enabled
            && is_placeholder_title(Some(&stored_title));
        let mut title = stored_title;
        if title.is_empty() {
            title = derive_provisional_title(&prompt);
        }
        if !title.is_empty() {
            yield StreamEvent::ChatTitle {
                title: title.clone(),
                chat_id: Some(chat_id.clone()),
                message_id: None,
                timestamp: Some(Utc::now()),
            };
        }

        let stored_results: Vec<(String, String)> = tool_results
            .iter()
            .map(|(id, _, output)| (id.clone(), output.clone()))
            .collect();
        let messages = display_messages(
            &incoming,
            &prompt,
            &assistant,
            "assistant_turn",
            &reasoning,
            &tool_calls,
            &tool_results,
            &confirming_ids,
        );
        let conversation = conversation_json(
            &full_history,
            &prompt,
            &assistant,
            &reasoning,
            &tool_calls,
            &stored_results,
        );
        let _ = state
            .store
            .checkpoint(Checkpoint {
                visitor_id: visitor_id.clone(),
                chat_id: chat_id.clone(),
                title: title.clone(),
                preview: preview_from(&messages, &prompt),
                messages,
                conversation_history: conversation.clone(),
                tokens_used: tokens,
            })
            .await;
        let _ = state
            .usage
            .record(UsageEvent {
                visitor_id: visitor_id.clone(),
                chat_id: chat_id.clone(),
                model: model_id.clone(),
                status: if errored { "failed".into() } else { "completed".into() },
                prompt_tokens,
                completion_tokens,
                tokens_used: tokens,
            })
            .await;

        if should_generate_title {
            let generated = generate_title(state.llm.clone(), &catalog, &prompt).await;
            if !generated.is_empty() && generated != title {
                title = generated;
                yield StreamEvent::ChatTitle {
                    title: title.clone(),
                    chat_id: Some(chat_id.clone()),
                    message_id: None,
                    timestamp: Some(Utc::now()),
                };
                let _ = state.store.rename(&visitor_id, &chat_id, &title).await;
            }
        }

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

fn confirming_pending(
    runtime: &ToolRuntime,
    conversation: &[serde_json::Value],
) -> Vec<PendingToolCall> {
    pending_tool_calls(conversation)
        .into_iter()
        .filter(|call| runtime.requires_confirmation(&call.name))
        .collect()
}

fn confirmation_response(message: &str) -> Box<axum::response::Response> {
    Box::new(
        (
            axum::http::StatusCode::CONFLICT,
            axum::Json(serde_json::json!({ "message": message })),
        )
            .into_response(),
    )
}

async fn resolve_tool_decisions(
    runtime: &ToolRuntime,
    pending: &[PendingToolCall],
    decisions: &[ToolDecision],
) -> Vec<StreamEvent> {
    let mut seen = HashSet::new();
    let mut events = Vec::new();
    for decision in decisions {
        if !seen.insert(decision.tool_call_id.clone()) {
            continue;
        }
        let Some(call) = pending.iter().find(|call| call.id == decision.tool_call_id) else {
            continue;
        };
        let normalized = decision.decision.trim().to_ascii_lowercase();
        let output = if normalized == "approve" {
            let outcome = runtime.execute(&call.name, call.arguments.clone()).await;
            for event in outcome.events {
                events.push(event);
            }
            normalize_output(outcome.output)
        } else if normalized == "deny" {
            denied_tool_output()
        } else {
            continue;
        };
        let links = resource_links(&output);
        events.push(StreamEvent::ToolResult {
            tool_call_id: call.id.clone(),
            tool_name: call.name.clone(),
            output,
            render_hint: links.as_ref().map(|_| "resource_links".to_string()),
            render_data: links,
            chat_id: None,
            message_id: None,
            timestamp: Some(Utc::now()),
        });
    }
    events
}

pub fn sse_done() -> axum::response::sse::Event {
    axum::response::sse::Event::default().data("[DONE]")
}

pub fn sse_event(event: &StreamEvent) -> Result<axum::response::sse::Event, Infallible> {
    Ok(axum::response::sse::Event::default().data(serde_json::to_string(event).expect("event")))
}
