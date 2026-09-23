use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_llm::{LlmChunk, LlmClient, ScriptedClient};
use sveda_protocol::{parse_sse_line, StreamEvent, HEADER_EMBED_TOKEN};
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::token;
use crate::{app, AppState, Config};

fn config() -> Config {
    let mut config = Config::test();
    config.web_enabled = false;
    config.title_generation_enabled = false;
    config.compaction_enabled = false;
    config
}

async fn send(state: AppState, token: &str, body: Value) -> (StatusCode, Vec<StreamEvent>) {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(header::ACCEPT, "text/event-stream".parse().unwrap());
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let mut http = Request::builder().method("POST").uri("/sveda/stream");
    for (name, value) in headers.iter() {
        http = http.header(name, value);
    }
    let response = app(state)
        .oneshot(
            http.body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let events = String::from_utf8(bytes.to_vec())
        .unwrap()
        .lines()
        .filter_map(parse_sse_line)
        .collect();
    (status, events)
}

async fn host(calls: Arc<Mutex<Vec<Value>>>) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/mcp/sveda"))
        .respond_with(move |request: &wiremock::Request| {
            let payload: Value = serde_json::from_slice(&request.body).unwrap_or(json!({}));
            let id = payload.get("id").cloned().unwrap_or(json!(1));
            let method = payload.get("method").and_then(Value::as_str).unwrap_or("");
            if method == "tools/call" {
                calls.lock().expect("calls").push(payload.clone());
            }
            match method {
                "initialize" => ResponseTemplate::new(200).set_body_json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "host", "version": "0.1.0" }
                    }
                })),
                "tools/list" => ResponseTemplate::new(200).set_body_json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [{
                            "name": "delete_post",
                            "description": "Delete a post",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "id": { "type": "string" } }
                            },
                            "_meta": {
                                "domain": "posts",
                                "mode": "delete",
                                "confirmation": "required"
                            }
                        }]
                    }
                })),
                "tools/call" => ResponseTemplate::new(200).set_body_json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [{ "type": "text", "text": "{\"success\":true,\"data\":{\"deleted\":true}}" }],
                        "isError": false
                    }
                })),
                _ => ResponseTemplate::new(202),
            }
        })
        .mount(&server)
        .await;
    server
}

fn tool_turn() -> Vec<Result<LlmChunk, sveda_llm::LlmError>> {
    vec![
        Ok(LlmChunk::ToolCall {
            id: "call-delete".into(),
            name: "delete_post".into(),
            input: json!({ "id": "1" }),
        }),
        Ok(LlmChunk::Usage {
            prompt_tokens: 3,
            completion_tokens: 2,
        }),
        Ok(LlmChunk::End {
            finish_reason: "tool_calls".into(),
        }),
    ]
}

fn text_turn(text: &str) -> Vec<Result<LlmChunk, sveda_llm::LlmError>> {
    vec![
        Ok(LlmChunk::TextDelta(text.into())),
        Ok(LlmChunk::Usage {
            prompt_tokens: 2,
            completion_tokens: 2,
        }),
        Ok(LlmChunk::End {
            finish_reason: "stop".into(),
        }),
    ]
}

#[tokio::test]
async fn approval_executes_stored_arguments_once() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server = host(calls.clone()).await;
    let llm = Arc::new(ScriptedClient::queue(vec![
        tool_turn(),
        text_turn("Deleted."),
        text_turn("Already handled."),
    ]));
    let state = AppState::with_llm(config(), llm.clone() as Arc<dyn LlmClient>);
    state.inject_host_mcp(
        "visitor-confirm",
        &format!("{}/mcp/sveda", server.uri()),
        "mcp-secret",
    );
    let token = token::issue(&state.config.hmac_key, "visitor-confirm", 120);

    let (status, events) = send(
        state.clone(),
        &token,
        json!({
            "prompt": "Delete post 1",
            "messages": [{ "role": "user", "content": "Delete post 1" }],
            "chatId": "confirm-chat"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    match events
        .iter()
        .find(|event| event.event_type() == "tool.call")
    {
        Some(StreamEvent::ToolCall {
            confirmation,
            tool_name,
            input,
            ..
        }) => {
            assert_eq!(tool_name, "delete_post");
            assert_eq!(confirmation.as_deref(), Some("required"));
            assert_eq!(input["id"], "1");
        }
        other => panic!("expected tool.call, got {other:?}"),
    }
    assert!(!events
        .iter()
        .any(|event| event.event_type() == "tool.result"));
    assert!(calls.lock().expect("calls").is_empty());

    let (status, events) = send(
        state.clone(),
        &token,
        json!({
            "prompt": "tool_results",
            "chatId": "confirm-chat",
            "messages": [{
                "role": "assistant",
                "tool_calls": [{
                    "id": "call-delete",
                    "function": { "name": "delete_post", "arguments": "{\"id\":\"999\"}" }
                }]
            }],
            "toolDecisions": [{ "toolCallId": "call-delete", "decision": "approve" }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(events.iter().any(|event| {
        matches!(
            event,
            StreamEvent::ToolResult { tool_name, output, .. }
                if tool_name == "delete_post" && output["success"] == true
        )
    }));
    let recorded = calls.lock().expect("calls");
    assert_eq!(recorded.len(), 1);
    assert_eq!(
        recorded[0]["params"]["arguments"]["id"], "1",
        "approval must use the stored arguments"
    );
    drop(recorded);

    let (status, _) = send(
        state,
        &token,
        json!({
            "prompt": "again",
            "chatId": "confirm-chat",
            "messages": [{ "role": "user", "content": "again" }],
            "toolDecisions": [{ "toolCallId": "call-delete", "decision": "approve" }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(calls.lock().expect("calls").len(), 1);
}

#[tokio::test]
async fn denial_does_not_call_the_host() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server = host(calls.clone()).await;
    let llm = Arc::new(ScriptedClient::queue(vec![tool_turn(), text_turn("Okay.")]));
    let state = AppState::with_llm(config(), llm as Arc<dyn LlmClient>);
    state.inject_host_mcp(
        "visitor-deny",
        &format!("{}/mcp/sveda", server.uri()),
        "mcp-secret",
    );
    let token = token::issue(&state.config.hmac_key, "visitor-deny", 120);
    let (status, _) = send(
        state.clone(),
        &token,
        json!({
            "prompt": "Delete post 1",
            "messages": [{ "role": "user", "content": "Delete post 1" }],
            "chatId": "deny-chat"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, events) = send(
        state,
        &token,
        json!({
            "chatId": "deny-chat",
            "toolDecisions": [{ "toolCallId": "call-delete", "decision": "deny" }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    match events
        .iter()
        .find(|event| event.event_type() == "tool.result")
    {
        Some(StreamEvent::ToolResult { output, .. }) => {
            assert_eq!(output["denied"], true);
            assert_eq!(output["success"], false);
        }
        other => panic!("expected tool.result, got {other:?}"),
    }
    assert!(calls.lock().expect("calls").is_empty());
}
