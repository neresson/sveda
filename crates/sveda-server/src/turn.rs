use std::collections::{BTreeMap, HashSet};
use std::convert::Infallible;
use std::pin::Pin;
use std::time::Instant;

use async_stream::stream;
use axum::response::IntoResponse;
use chrono::Utc;
use futures_util::{Stream, StreamExt};
use serde_json::{json, Value};
use sveda_agent::{
    conversation_json, denied_tool_output, derive_provisional_title, display_messages,
    generate_summary, generate_title, inject_summary, is_placeholder_title, json_to_chat,
    normalize_output, pending_tool_calls, preview_from, resource_links, should_compact,
    split_prompt_and_history, tail_messages, AgentConfig, AgentRequest, ApprovalOffer,
    PendingToolCall, ToolRuntime,
};
use sveda_llm::{Catalog, ChatMessage, StoredToolCall};
use sveda_mcp::{HostMcpClient, McpCallContext, McpCredentials};
use sveda_protocol::diagnostics::{identifier, log, Context, Level, CONTEXT};
use sveda_protocol::{ConfirmationScope, StreamEvent, StreamRequest, ToolDecision, ToolTarget};
use sveda_store::{Checkpoint, HistoryRecord, StoreError, UsageEvent};
use uuid::Uuid;

use crate::admin_tools;
use crate::policy::{
    apply_tool_approval_policy, claims_restricted, filter_client_tools, filter_mcp_tools,
    resolve_effective, EffectiveCapabilities,
};
use crate::token::TokenClaims;
use crate::tool_approvals::{chat_key, fingerprint, ChatApprovals};
use crate::web;
use crate::AppState;

const TOOL_ACCESS_INSTRUCTIONS: &str = "Use only tools exposed for this user. Never say a tool or feature is temporarily disabled, unavailable, or under maintenance. If no tool fits after checking the available tools, say you do not have access to that action.";

pub async fn run_turn(
    state: AppState,
    claims: TokenClaims,
    request: StreamRequest,
) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, Box<axum::response::Response>> {
    let chat_id = request
        .chat_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("chat_{}", Uuid::new_v4()));
    CONTEXT
        .scope(
            Context {
                chat_id: chat_id.clone(),
                turn_id: Uuid::new_v4().to_string(),
                ..Context::default()
            },
            async {
                let result = run_turn_inner(state, claims, request, chat_id).await;
                if let Err(response) = &result {
                    log(
                        Level::Warn,
                        "turn.rejected",
                        || json!({"http_status": response.status().as_u16()}),
                    );
                }
                result
            },
        )
        .await
}

async fn run_turn_inner(
    state: AppState,
    claims: TokenClaims,
    request: StreamRequest,
    chat_id: String,
) -> Result<Pin<Box<dyn Stream<Item = StreamEvent> + Send>>, Box<axum::response::Response>> {
    let diagnostics = Context::current();
    let started = Instant::now();
    let visitor_id = claims.visitor_id.clone();
    let admin = claims.admin;
    log(Level::Info, "turn.started", || {
        json!({
            "resume": !request.tool_decisions.is_empty(),
            "decisions": request.tool_decisions.len(),
        })
    });
    log(Level::Debug, "turn.request", || {
        json!({
            "has_prompt": request.prompt.as_ref().is_some_and(|prompt| !prompt.trim().is_empty()),
            "decisions": request.tool_decisions.iter().take(32).map(|decision| json!({
                "tool_call_id": identifier(&decision.tool_call_id),
                "decision": match decision.decision.trim().to_ascii_lowercase().as_str() {
                    "approve" => "approve",
                    "approve_batch" => "approve_batch",
                    "approve_for_chat" => "approve_for_chat",
                    "deny" => "deny",
                    _ => "unknown",
                },
            })).collect::<Vec<_>>(),
        })
    });
    let settings = state.settings.document();
    let mut lease = if !settings.tool_approvals.is_empty() || !request.tool_decisions.is_empty() {
        Some(
            state
                .approval_turns
                .acquire(&chat_key(&visitor_id, &chat_id))
                .map_err(|_| confirmation_response("Another turn is in progress for this chat."))?,
        )
    } else {
        None
    };
    let mut incoming = request.messages.clone().unwrap_or_default();
    let (mut prompt, mut history) = split_prompt_and_history(request.prompt.as_deref(), &incoming);
    let stored = state
        .store
        .record(&visitor_id, &chat_id)
        .await
        .map_err(|_| confirmation_response("Chat history is unavailable."))?;
    let grants = stored.as_ref().map(|record| {
        ChatApprovals::new(
            state.mcp.clone(),
            &visitor_id,
            &chat_id,
            record.created_at.timestamp_micros(),
        )
    });
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
    let lms_visitor = claims.is_lms_visitor();
    let (catalog, requested_model) = if lms_visitor {
        (settings.lms_chat_catalog(), None)
    } else {
        (state.catalog(), request.model)
    };
    let model_id = catalog
        .resolve(requested_model.as_deref())
        .map(|model| model.id.clone())
        .unwrap_or_default();
    let mut full_history = history.clone();
    log(Level::Debug, "turn.model_config", || {
        json!({
            "model": identifier(&model_id), "stream_timeout_s": runtime_config.stream_timeout,
            "host_mcp_timeout_s": runtime_config.mcp_timeout,
        })
    });
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
    if lms_visitor {
        instructions = sveda_agent::append_instruction(
            instructions,
            "In user-facing replies, introduce yourself as the Sveda assistant. Keep the underlying model and provider internal.",
        );
    }

    let restricted = !admin && claims_restricted(claims.policy.as_deref(), claims.grants.as_ref());
    let capabilities = if admin {
        EffectiveCapabilities::unrestricted()
    } else if restricted {
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
            let tools = client.list_tools().await.map_err(|_| {
                log(Level::Warn, "turn.host_tools_unavailable", || json!({
                    "elapsed_ms": started.elapsed().as_millis(),
                }));
                Box::new((
                    axum::http::StatusCode::BAD_GATEWAY,
                    axum::Json(serde_json::json!({"code":"host_tools_unavailable","message":"Unable to load tools from the host application. Try again after the connection is restored."})),
                ).into_response())
            })?;
            let host_tool_count = tools.len();
            let mut tools = filter_mcp_tools(tools, &capabilities.mcp, restricted);
            // Host instructions are free text and can describe tools filtered out by the sidecar.
            let server_instructions = if tools.len() == host_tool_count {
                client.instructions()
            } else {
                None
            };
            let options = tools
                .iter_mut()
                .filter_map(|tool| {
                    let policy = settings.tool_approvals.get(&tool.name).copied();
                    apply_tool_approval_policy(tool, policy).map(|offer| (tool.name.clone(), offer))
                })
                .collect();
            let chat_grants = grants.clone();
            runtime = ToolRuntime::mcp(std::sync::Arc::new(client), tools)
                .with_defer(runtime_config.defer_enabled, runtime_config.defer_min_pool)
                .with_approvals(
                    options,
                    std::sync::Arc::new(move |host, tool, scope| {
                        chat_grants
                            .as_ref()
                            .is_some_and(|grants| grants.permits(host, tool, scope))
                    }),
                );
            let names = runtime
                .advertised()
                .into_iter()
                .map(|tool| tool.name)
                .filter(|name| name != sveda_agent::SPAWN_TOOL_NAME)
                .collect::<Vec<_>>();
            let host_instructions =
                sveda_agent::host_tools_instruction(server_instructions.as_deref(), &names);
            instructions = sveda_agent::append_instruction(instructions, &host_instructions);
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

    instructions = sveda_agent::append_instruction(instructions, TOOL_ACCESS_INSTRUCTIONS);

    let filtered_client_tools = filter_client_tools(
        request.client_tools.as_ref(),
        &capabilities.client,
        restricted,
    );
    let client_tool_specs = sveda_agent::client_tool_specs(filtered_client_tools.as_ref());
    let tool_decisions = request.tool_decisions.clone();
    let enhanced_decision = tool_decisions.iter().any(|decision| {
        let decision = decision.decision.trim();
        decision.eq_ignore_ascii_case("approve_batch")
            || decision.eq_ignore_ascii_case("approve_for_chat")
    });
    let mut decision_events = Vec::new();
    let approval_host = runtime.approval_host().map(ToOwned::to_owned);
    let mut skip_model = false;
    if !tool_decisions.is_empty() && stored.is_none() {
        return Err(confirmation_response("Chat history is unavailable."));
    }
    if let Some(record) = stored.as_ref() {
        let pending = confirming_pending(&runtime, &record.conversation_history);
        if enhanced_decision && pending.is_empty() {
            return Err(confirmation_response(
                "These tool approvals are no longer pending. Reload the chat.",
            ));
        }
        if !pending.is_empty() && tool_decisions.is_empty() {
            return Err(confirmation_response(
                "Tool confirmation is required before the conversation can continue.",
            ));
        }
        if !pending.is_empty() {
            // Persist decisions independently of the HTTP connection before continuing
            // the model. The chat lease also stays with the task on disconnect.
            let task_state = state.clone();
            let record = record.clone();
            let pending_calls = pending.clone();
            let (returned_runtime, returned_lease, updated, events) =
                tokio::spawn(CONTEXT.scope(diagnostics.clone(), async move {
                    let (updated, events) = resolve_tool_decisions(
                        &task_state,
                        &runtime,
                        record,
                        &pending_calls,
                        &tool_decisions,
                        grants.as_ref(),
                    )
                    .await?;
                    Ok::<_, StoreError>((runtime, lease, updated, events))
                }))
                .await
                .ok()
                .and_then(Result::ok)
                .ok_or_else(|| confirmation_response("Could not save tool decisions."))?;
            runtime = returned_runtime;
            lease = returned_lease;
            decision_events = events;
            if enhanced_decision && decision_events.is_empty() {
                return Err(confirmation_response(
                    "The selected actions need individual confirmation or are no longer pending.",
                ));
            }
            let resolved: HashSet<_> = decision_events
                .iter()
                .filter_map(|event| match event {
                    StreamEvent::ToolResult { tool_call_id, .. } => Some(tool_call_id.as_str()),
                    _ => None,
                })
                .collect();
            history = updated
                .conversation_history
                .iter()
                .filter_map(json_to_chat)
                .collect();
            full_history = history.clone();
            if updated.summary.as_deref().is_some() {
                history = tail_messages(&history, runtime_config.compaction_keep_tail);
            }
            incoming = updated.messages;
            prompt = String::new();
            skip_model = pending
                .iter()
                .any(|call| !resolved.contains(call.id.as_str()));
            log(Level::Info, "turn.approvals_resolved", || {
                json!({
                    "pending": pending.len(), "resolved": resolved.len(),
                    "continue_to_model": !skip_model, "elapsed_ms": started.elapsed().as_millis(),
                })
            });
        }
    }

    if skip_model {
        return Ok(Box::pin(stream! {
            let _lease = lease;
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
        }));
    }

    let agent_request = AgentRequest {
        prompt: prompt.clone(),
        history,
        chat_id: Some(chat_id.clone()),
        model: requested_model,
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
    let events = sveda_agent::run_with_tools(
        state.llm.clone(),
        catalog.clone(),
        AgentConfig {
            max_steps: runtime_config.max_steps,
            context_max_tokens: runtime_config.context_max_tokens,
        },
        agent_request,
        runtime,
    )
    .map_err(|error| {
        Box::new(
            (
                axum::http::StatusCode::UNPROCESSABLE_ENTITY,
                axum::Json(json!({ "message": error.to_string() })),
            )
                .into_response(),
        )
    })?;
    let mut finalized = Box::pin(finalize_turn(
        state,
        visitor_id,
        chat_id,
        model_id,
        catalog,
        prompt,
        incoming,
        full_history,
        approval_host,
        Box::pin(events),
    ));
    Ok(Box::pin(stream! {
        let _lease = lease;
        for event in decision_events {
            yield event;
        }
        while let Some(mut event) = CONTEXT.scope(diagnostics.clone(), finalized.next()).await {
            if lms_visitor {
                if let StreamEvent::Error { message, .. } = &mut event {
                    *message = "Sveda could not complete the request. Please try again.".into();
                }
            }
            if let StreamEvent::MessageEnd { finish_reason, usage, message_id, .. } = &event {
                CONTEXT.sync_scope(Context {
                    message_id: message_id.clone().unwrap_or_default(),
                    ..diagnostics.clone()
                }, || log(Level::Info, "turn.finished", || json!({
                    "finish_reason": finish_reason.as_deref().map(identifier), "usage": usage,
                    "elapsed_ms": started.elapsed().as_millis(),
                })));
            }
            yield event;
        }
    }))
}

fn finalize_turn(
    state: AppState,
    visitor_id: String,
    chat_id: String,
    model_id: String,
    catalog: Catalog,
    prompt: String,
    incoming: Vec<serde_json::Value>,
    full_history: Vec<ChatMessage>,
    approval_host: Option<String>,
    events: Pin<Box<dyn Stream<Item = StreamEvent> + Send>>,
) -> impl Stream<Item = StreamEvent> + Send {
    stream! {
        let mut events = events;
        let mut assistant = String::new();
        let mut assistant_id = None;
        let mut reasoning = String::new();
        let mut tool_calls: Vec<StoredToolCall> = Vec::new();
        let mut confirming_ids = Vec::new();
        let mut backend_pending = HashSet::new();
        let mut approval_offers = BTreeMap::new();
        let mut tool_results: Vec<(String, String, String)> = Vec::new();
        let mut tokens = 0u64;
        let mut prompt_tokens = 0u64;
        let mut completion_tokens = 0u64;
        let mut errored = false;
        while let Some(event) = events.next().await {
            match &event {
                StreamEvent::MessageStart { message_id, .. } => assistant_id = message_id.clone(),
                StreamEvent::TextDelta { delta, .. } => assistant.push_str(delta),
                StreamEvent::ReasoningDelta { delta, .. } => reasoning.push_str(delta),
                StreamEvent::ToolCall {
                    tool_call_id,
                    tool_name,
                    input,
                    confirmation,
                    confirmation_scope,
                    approval_scope_key,
                    target,
                    ..
                } => {
                    if confirmation.as_deref() == Some("required") {
                        confirming_ids.push(tool_call_id.clone());
                        if *target == ToolTarget::Backend {
                            backend_pending.insert(tool_call_id.clone());
                        }
                    }
                    if let (Some(scope), Some(host)) = (confirmation_scope, approval_host.as_deref()) {
                        approval_offers.insert(
                            tool_call_id.clone(),
                            (*scope, approval_fingerprint(host, approval_scope_key.as_deref().unwrap_or(""), *scope)),
                        );
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
        let mut messages = display_messages(
            &incoming,
            &prompt,
            &assistant,
            assistant_id.as_deref().unwrap_or("assistant_turn"),
            &reasoning,
            &tool_calls,
            &tool_results,
            &confirming_ids,
        );
        let mut conversation = conversation_json(
            &full_history,
            &prompt,
            &assistant,
            &reasoning,
            &tool_calls,
            &stored_results,
        );
        if let Some(calls) = conversation.iter_mut().rev().find_map(|msg| msg["tool_calls"].as_array_mut()) {
            for call in calls {
                let id = call["id"].as_str().unwrap_or_default();
                let pending = backend_pending.contains(id);
                if let Some((_, key)) = approval_offers.get(id) {
                    call["approval"] = json!(key);
                }
                if pending {
                    call["confirmation"] = json!("required");
                }
            }
        }
        if let Some(parts) = messages.last_mut().and_then(|message| message["parts"].as_array_mut()) {
            for part in parts.iter_mut().filter(|part| part["type"] == "tool-call") {
                if let Some((scope, _)) = part["toolCallId"].as_str().and_then(|id| approval_offers.get(id)) {
                    part["confirmationScope"] = json!(scope);
                }
            }
        }
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
        .filter(|call| {
            call.approval_started
                || call.confirmation_required
                || runtime.requires_confirmation(&call.name)
        })
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

fn approval_fingerprint(host: &str, scope: &str, kind: ConfirmationScope) -> String {
    let key = fingerprint(&[host, scope]);
    if kind == ConfirmationScope::Course {
        format!("course:{key}")
    } else {
        key
    }
}

async fn pending_approval_offer(
    runtime: &ToolRuntime,
    call: &PendingToolCall,
) -> Option<ApprovalOffer> {
    let offered = call.approval.as_deref()?;
    let host = runtime.approval_host()?;
    let offer = runtime.approval_offer(&call.name, &call.arguments).await?;
    let scope = match &offer {
        ApprovalOffer::Course(scope) | ApprovalOffer::Chat(scope) => scope.as_str(),
        ApprovalOffer::Batch => "",
        ApprovalOffer::Auto(_) => return None,
    };
    (offered == approval_fingerprint(host, scope, offer.scope()?)).then_some(offer)
}

enum ApprovedAction {
    Deny,
    Run,
    Guarded {
        scope: String,
        legacy: bool,
        grant: bool,
    },
    CoveredCourse(String),
    UnknownOutcome,
}

async fn resolve_tool_decisions(
    state: &AppState,
    runtime: &ToolRuntime,
    mut record: HistoryRecord,
    pending: &[PendingToolCall],
    decisions: &[ToolDecision],
    grants: Option<&ChatApprovals>,
) -> Result<(HistoryRecord, Vec<StreamEvent>), StoreError> {
    let mut seen = HashSet::new();
    let mut actions = Vec::new();
    let host = runtime.approval_host();
    for decision in decisions {
        if !seen.insert(decision.tool_call_id.as_str()) {
            continue;
        }
        let Some(call) = pending.iter().find(|call| call.id == decision.tool_call_id) else {
            continue;
        };
        let normalized = decision.decision.trim().to_ascii_lowercase();
        let action = match normalized.as_str() {
            "approve" | "approve_batch" | "approve_for_chat" | "deny" if call.approval_started => {
                ApprovedAction::UnknownOutcome
            }
            "deny" => ApprovedAction::Deny,
            "approve" if runtime.known(&call.name) => {
                if let Some(ApprovalOffer::Course(scope)) =
                    pending_approval_offer(runtime, call).await
                {
                    ApprovedAction::Guarded {
                        scope,
                        legacy: false,
                        grant: false,
                    }
                } else {
                    ApprovedAction::Run
                }
            }
            "approve_batch" | "approve_for_chat" => {
                match pending_approval_offer(runtime, call).await {
                    Some(ApprovalOffer::Batch) if normalized == "approve_batch" => {
                        ApprovedAction::Run
                    }
                    Some(ApprovalOffer::Chat(scope)) => ApprovedAction::Guarded {
                        scope,
                        legacy: true,
                        grant: normalized == "approve_for_chat",
                    },
                    Some(ApprovalOffer::Course(scope)) if normalized == "approve_for_chat" => {
                        ApprovedAction::Guarded {
                            scope,
                            legacy: false,
                            grant: true,
                        }
                    }
                    _ => continue,
                }
            }
            _ => continue,
        };
        let path = match &action {
            ApprovedAction::Deny => "deny",
            ApprovedAction::UnknownOutcome => "unknown_outcome",
            ApprovedAction::Run if normalized == "approve_batch" => "batch",
            ApprovedAction::Run => "plain",
            ApprovedAction::Guarded { .. } => "guarded",
            ApprovedAction::CoveredCourse(_) => "covered_course",
        };
        log(Level::Debug, "turn.approval_path", || {
            json!({
                "path": path, "tool_call_id": identifier(&call.id), "tool": identifier(&call.name),
            })
        });
        actions.push((call, action));
    }
    if actions.is_empty() {
        return Ok((record, Vec::new()));
    }

    // Calls proposed together can already be pending when a course is approved.
    // Honor explicit decisions first, then recheck any pending sibling's scope.
    let course_grants: HashSet<_> = actions
        .iter()
        .filter_map(|(_, action)| match action {
            ApprovedAction::Guarded {
                scope,
                legacy: false,
                grant: true,
            } => Some(scope.clone()),
            _ => None,
        })
        .collect();
    for call in pending.iter().filter(|call| {
        !seen.contains(call.id.as_str())
            && !call.approval_started
            && call
                .approval
                .as_deref()
                .is_some_and(|offered| offered.starts_with("course:"))
    }) {
        if let Some(ApprovalOffer::Course(scope)) = pending_approval_offer(runtime, call).await {
            if course_grants.contains(&scope) || runtime.has_chat_approval(None, &scope) {
                actions.push((call, ApprovedAction::CoveredCourse(scope)));
            }
        }
    }

    // Reserve writes in durable history before calling the host. A crash or a
    // lost response can leave the outcome unknown, but a retry cannot repeat it.
    let writes: HashSet<_> = actions
        .iter()
        .filter_map(|(call, action)| {
            matches!(
                action,
                ApprovedAction::Run
                    | ApprovedAction::Guarded { .. }
                    | ApprovedAction::CoveredCourse(_)
            )
            .then_some(call.id.as_str())
        })
        .collect();
    if !writes.is_empty() {
        set_approval_started(&mut record.conversation_history, &writes, true);
        record = checkpoint_decisions(state, record).await?;
    }
    let mut events = Vec::new();
    let mut results = Vec::new();
    let mut new_grants = Vec::new();
    for (call, action) in actions {
        let output = match action {
            ApprovedAction::Deny => denied_tool_output(),
            ApprovedAction::UnknownOutcome => json!({
                "success": false,
                "error": "An earlier approval may have executed this action. Verify its outcome before retrying."
            }),
            ApprovedAction::Run => {
                let outcome = runtime
                    .execute_call(&call.name, call.arguments.clone(), Some(&call.id))
                    .await;
                events.extend(outcome.events);
                normalize_output(outcome.output)
            }
            ApprovedAction::Guarded {
                scope,
                legacy,
                grant,
            } => {
                let outcome = match runtime
                    .execute_guarded(&call.name, &call.arguments, &scope, &call.id, legacy)
                    .await
                {
                    Some(outcome) => outcome,
                    None if !legacy && !grant => {
                        // The user explicitly approved this call and the host did not write.
                        runtime
                            .execute_call(&call.name, call.arguments.clone(), Some(&call.id))
                            .await
                    }
                    None => continue,
                };
                let output = normalize_output(outcome.output);
                if grant && output["success"] == true {
                    new_grants.push((legacy.then_some(call.name.as_str()), scope));
                }
                output
            }
            ApprovedAction::CoveredCourse(scope) => {
                if !runtime.has_chat_approval(None, &scope)
                    && !new_grants
                        .iter()
                        .any(|(tool, granted)| tool.is_none() && *granted == scope)
                {
                    continue;
                }
                let Some(outcome) = runtime
                    .execute_guarded(&call.name, &call.arguments, &scope, &call.id, false)
                    .await
                else {
                    continue;
                };
                normalize_output(outcome.output)
            }
        };
        let links = resource_links(&output);
        results.push((call.id.clone(), call.name.clone(), output.to_string()));
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
    set_approval_started(&mut record.conversation_history, &writes, false);
    record.messages = display_messages(&record.messages, "", "", "", "", &[], &results, &[]);
    record.conversation_history.extend(
        results.into_iter().map(
            |(id, _, output)| json!({ "role": "tool", "tool_call_id": id, "content": output }),
        ),
    );
    let record = checkpoint_decisions(state, record).await?;
    if let Some((grants, host)) = grants.zip(host) {
        for (name, scope) in new_grants {
            if grants.grant(host, name, &scope).is_err() {
                // The approved call ran, but a failed grant write must leave
                // future calls requiring confirmation.
                break;
            }
        }
    }
    Ok((record, events))
}

fn set_approval_started(conversation: &mut [Value], ids: &HashSet<&str>, started: bool) {
    for call in conversation
        .iter_mut()
        .filter_map(|msg| msg["tool_calls"].as_array_mut())
        .flatten()
    {
        if call["id"].as_str().is_some_and(|id| ids.contains(id)) {
            call["approvalStarted"] = json!(started);
        }
    }
}

async fn checkpoint_decisions(
    state: &AppState,
    record: HistoryRecord,
) -> Result<HistoryRecord, StoreError> {
    state
        .store
        .checkpoint(Checkpoint {
            visitor_id: record.visitor_id,
            chat_id: record.chat_id,
            title: record.title,
            preview: record.preview,
            messages: record.messages,
            conversation_history: record.conversation_history,
            tokens_used: 0,
        })
        .await
}

pub fn sse_done() -> axum::response::sse::Event {
    axum::response::sse::Event::default().data("[DONE]")
}

pub fn sse_event(event: &StreamEvent) -> Result<axum::response::sse::Event, Infallible> {
    Ok(axum::response::sse::Event::default().data(serde_json::to_string(event).expect("event")))
}
