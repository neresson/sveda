use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_llm::{LlmChunk, LlmClient, ScriptedClient};
use sveda_protocol::{parse_sse_line, StreamEvent, ADMIN_VISITOR_ID, HEADER_EMBED_TOKEN};
use sveda_server::{app, AppState, Config};
use sveda_store::UsageEvent;
use tower::ServiceExt;

static CHAT: AtomicU64 = AtomicU64::new(1);

fn next_chat() -> String {
    format!("admin-chat-{}", CHAT.fetch_add(1, Ordering::Relaxed))
}

fn admin_config() -> Config {
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    config
}

fn indexed_config() -> (Config, PathBuf) {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("sveda-admin-index-{nanos}"));
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "pub fn occupancy() {}\n").unwrap();
    let mut config = admin_config();
    config.index_root = Some(root.clone());
    (config, root)
}

async fn send(
    state: AppState,
    method: &str,
    uri: &str,
    headers: HeaderMap,
    body: Body,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let request = builder.body(body).expect("request");
    let response = app(state).oneshot(request).await.expect("response");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    (status, headers, bytes)
}

fn json_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    headers
}

fn cookie_from(headers: &HeaderMap) -> String {
    headers
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .unwrap_or_default()
        .to_string()
}

async fn login_cookie(state: AppState) -> String {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded".parse().unwrap(),
    );
    let (status, response_headers, _) = send(
        state,
        "POST",
        "/admin/login",
        headers,
        Body::from("key=sveda-admin-secret"),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    cookie_from(&response_headers)
}

async fn admin_session_token(state: AppState) -> String {
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::HOST, "127.0.0.1:8787".parse().unwrap());
    let (status, _, body) = send(state, "POST", "/admin/session", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

async fn public_token(state: AppState, visitor_id: &str) -> String {
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(serde_json::to_vec(&json!({ "visitor_id": visitor_id })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

fn tool_then_text(name: &str, input: Value, text: &str) -> ScriptedClient {
    ScriptedClient::queue(vec![
        vec![
            Ok(LlmChunk::ToolCall {
                id: "call-1".into(),
                name: name.into(),
                input,
            }),
            Ok(LlmChunk::Usage {
                prompt_tokens: 8,
                completion_tokens: 2,
            }),
            Ok(LlmChunk::End {
                finish_reason: "tool_calls".into(),
            }),
        ],
        vec![
            Ok(LlmChunk::TextDelta(text.into())),
            Ok(LlmChunk::Usage {
                prompt_tokens: 10,
                completion_tokens: 4,
            }),
            Ok(LlmChunk::End {
                finish_reason: "stop".into(),
            }),
        ],
    ])
}

fn hello_world() -> ScriptedClient {
    ScriptedClient::queue(vec![vec![
        Ok(LlmChunk::TextDelta("ok".into())),
        Ok(LlmChunk::Usage {
            prompt_tokens: 1,
            completion_tokens: 1,
        }),
        Ok(LlmChunk::End {
            finish_reason: "stop".into(),
        }),
    ]])
}

async fn stream(
    state: AppState,
    token: &str,
    prompt: &str,
    chat_id: &str,
    context: Value,
) -> (StatusCode, Vec<StreamEvent>) {
    let mut headers = json_headers();
    headers.insert("accept", "text/event-stream".parse().unwrap());
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "prompt": prompt,
                "messages": [{ "id": "m1", "role": "user", "content": prompt }],
                "chatId": chat_id,
                "context": context
            }))
            .unwrap(),
        ),
    )
    .await;
    let events: Vec<StreamEvent> = String::from_utf8(body)
        .unwrap()
        .lines()
        .filter_map(parse_sse_line)
        .collect();
    (status, events)
}

fn tool_result(events: &[StreamEvent]) -> Value {
    events
        .iter()
        .find_map(|event| match event {
            StreamEvent::ToolResult { output, .. } => Some(output.clone()),
            _ => None,
        })
        .expect("tool result")
}

fn advertised(llm: &ScriptedClient) -> Vec<String> {
    llm.last_tools.lock().expect("tools")[0].clone()
}

fn last_instructions(llm: &ScriptedClient) -> String {
    llm.last_instructions
        .lock()
        .expect("instructions")
        .clone()
        .unwrap_or_default()
}

const ADMIN_TOOLS: &[&str] = &[
    "admin_dashboard",
    "admin_usage",
    "admin_get_settings",
    "admin_update_settings",
    "admin_upsert_model",
    "admin_remove_model",
    "admin_open_page",
];

#[tokio::test]
async fn embed_token_rejects_reserved_admin_visitor() {
    for visitor_id in [ADMIN_VISITOR_ID, "SVEDA-ADMIN", "Sveda-Admin"] {
        let (status, _, body) = send(
            AppState::new(admin_config()),
            "POST",
            "/sveda/embed/token",
            json_headers(),
            Body::from(serde_json::to_vec(&json!({ "visitor_id": visitor_id })).unwrap()),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{visitor_id}");
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["message"], "visitor_id is reserved");
    }
}

#[tokio::test]
async fn admin_session_advertises_only_operator_tools() {
    let llm = Arc::new(hello_world());
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    let token = admin_session_token(state.clone()).await;
    let (status, _) = stream(
        state,
        &token,
        "Hello",
        &next_chat(),
        json!({ "admin": { "page": "models" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tools = advertised(&llm);
    assert!(tools.contains(&"spawn_tasks".to_string()));
    for name in ADMIN_TOOLS {
        assert!(tools.contains(&name.to_string()), "{name}");
    }
    assert!(!tools.contains(&"search_code".to_string()));
    assert!(!tools.contains(&"read_code_file".to_string()));
    assert!(!tools.contains(&"search_agent_tools".to_string()));
    assert!(!tools.contains(&"web_search".to_string()));
    assert!(!tools.contains(&"web_fetch".to_string()));
    let instructions = last_instructions(&llm);
    assert!(instructions.contains("admin copilot"));
    assert!(instructions.contains("currently on the models page"));
    assert!(!instructions.contains("shopping assistant"));
}

#[tokio::test]
async fn admin_chat_ignores_the_public_system_prompt() {
    let mut config = admin_config();
    config.system_prompt = "You are a shopping assistant named ShopBot.".into();
    let llm = Arc::new(hello_world());
    let state = AppState::with_llm(config, llm.clone() as Arc<dyn LlmClient>);
    let token = admin_session_token(state.clone()).await;
    let (status, _) = stream(
        state,
        &token,
        "Who are you?",
        &next_chat(),
        json!({ "admin": { "page": "dashboard" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let instructions = last_instructions(&llm);
    assert!(instructions.contains("admin copilot"));
    assert!(!instructions.contains("ShopBot"));
    assert!(!instructions.contains("shopping assistant named"));
}

#[tokio::test]
async fn public_chat_keeps_the_embed_system_prompt() {
    let mut config = admin_config();
    config.system_prompt = "You are a shopping assistant named ShopBot.".into();
    let llm = Arc::new(hello_world());
    let state = AppState::with_llm(config, llm.clone() as Arc<dyn LlmClient>);
    let token = public_token(state.clone(), "visitor-prompt").await;
    let (status, _) = stream(
        state,
        &token,
        "Who are you?",
        &next_chat(),
        json!({ "page": "checkout" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let instructions = last_instructions(&llm);
    assert!(instructions.contains("ShopBot"));
    assert!(instructions.contains("web_search"));
    assert!(!instructions.contains("admin copilot"));
}

#[tokio::test]
async fn admin_chat_can_read_dashboard_stats() {
    let llm = Arc::new(tool_then_text(
        "admin_dashboard",
        json!({}),
        "No traffic yet.",
    ));
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    state
        .usage()
        .record(UsageEvent {
            visitor_id: "u1".into(),
            chat_id: "c1".into(),
            model: "deepseek-v4-flash-responses".into(),
            status: "completed".into(),
            prompt_tokens: 6,
            completion_tokens: 4,
            tokens_used: 10,
        })
        .await
        .unwrap();
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state,
        &token,
        "Show stats",
        &next_chat(),
        json!({ "admin": { "page": "dashboard" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["success"], true);
    assert_eq!(output["page"], "dashboard");
    assert_eq!(output["reload"], false);
    assert_eq!(output["data"]["requests"], 1);
    assert_eq!(output["data"]["tokens_used"], 10);
}

#[tokio::test]
async fn admin_chat_can_read_usage() {
    let llm = Arc::new(tool_then_text(
        "admin_usage",
        json!({ "page": 1 }),
        "Usage loaded.",
    ));
    let state = AppState::with_llm(admin_config(), llm);
    state
        .usage()
        .record(UsageEvent {
            visitor_id: "u1".into(),
            chat_id: "c1".into(),
            model: "deepseek-v4-pro".into(),
            status: "completed".into(),
            prompt_tokens: 3,
            completion_tokens: 2,
            tokens_used: 5,
        })
        .await
        .unwrap();
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state,
        &token,
        "Show usage",
        &next_chat(),
        json!({ "admin": { "page": "usage" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["page"], "usage");
    assert_eq!(output["data"]["requests"]["total"], 1);
    assert_eq!(output["data"]["by_model"][0]["model"], "deepseek-v4-pro");
}

#[tokio::test]
async fn admin_chat_can_read_and_patch_settings() {
    let llm = Arc::new(tool_then_text(
        "admin_get_settings",
        json!({ "section": "prompts" }),
        "Prompts loaded.",
    ));
    let state = AppState::with_llm(admin_config(), llm);
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state.clone(),
        &token,
        "Show prompts",
        &next_chat(),
        json!({ "admin": { "page": "prompts" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["page"], "prompts");
    assert!(output["data"].get("welcome_message").is_some());
    assert!(output["data"].get("mcp").is_none());

    let llm = Arc::new(tool_then_text(
        "admin_update_settings",
        json!({
            "welcome_message": "Welcome back",
            "system_prompt": "Stay inside admin."
        }),
        "Prompts saved.",
    ));
    let state = AppState::with_llm(admin_config(), llm);
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state.clone(),
        &token,
        "Set the welcome message",
        &next_chat(),
        json!({ "admin": { "page": "prompts" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["success"], true);
    assert_eq!(output["page"], "prompts");
    assert_eq!(output["reload"], true);
    assert_eq!(output["data"]["welcome_message"], "Welcome back");

    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::ACCEPT, "application/json".parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin/settings", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let saved: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(saved["welcome_message"], "Welcome back");
}

#[tokio::test]
async fn admin_chat_can_update_mcp_settings() {
    let llm = Arc::new(tool_then_text(
        "admin_update_settings",
        json!({
            "mcp": {
                "mcpServers": {
                    "deepwiki": { "url": "https://mcp.deepwiki.com/mcp" }
                }
            }
        }),
        "MCP saved.",
    ));
    let state = AppState::with_llm(admin_config(), llm);
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state.clone(),
        &token,
        "Add DeepWiki MCP",
        &next_chat(),
        json!({ "admin": { "page": "mcp" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["success"], true);
    assert_eq!(output["page"], "mcp");
    assert_eq!(
        output["data"]["mcp"]["mcpServers"]["deepwiki"]["url"],
        "https://mcp.deepwiki.com/mcp"
    );

    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::ACCEPT, "application/json".parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin/settings", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let saved: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        saved["mcp"]["mcpServers"]["deepwiki"]["url"],
        "https://mcp.deepwiki.com/mcp"
    );
}

fn tool_call(name: &str, input: Value) -> Vec<Result<LlmChunk, sveda_llm::LlmError>> {
    vec![
        Ok(LlmChunk::ToolCall {
            id: "call-1".into(),
            name: name.into(),
            input,
        }),
        Ok(LlmChunk::Usage {
            prompt_tokens: 8,
            completion_tokens: 2,
        }),
        Ok(LlmChunk::End {
            finish_reason: "tool_calls".into(),
        }),
    ]
}

fn text_reply(text: &str) -> Vec<Result<LlmChunk, sveda_llm::LlmError>> {
    vec![
        Ok(LlmChunk::TextDelta(text.into())),
        Ok(LlmChunk::Usage {
            prompt_tokens: 10,
            completion_tokens: 4,
        }),
        Ok(LlmChunk::End {
            finish_reason: "stop".into(),
        }),
    ]
}

#[tokio::test]
async fn admin_chat_can_upsert_and_remove_a_model() {
    let llm = Arc::new(ScriptedClient::queue(vec![
        tool_call(
            "admin_upsert_model",
            json!({
                "id": "local-llama",
                "label": "Local Llama",
                "url": "http://127.0.0.1:8080",
                "key": "sk-local"
            }),
        ),
        text_reply("Model saved."),
        tool_call("admin_remove_model", json!({ "id": "local-llama" })),
        text_reply("Removed."),
    ]));
    let state = AppState::with_llm(admin_config(), llm);
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state.clone(),
        &token,
        "Add local llama",
        &next_chat(),
        json!({ "admin": { "page": "models" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["page"], "models");
    let created = output["data"]["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|model| model["id"] == "local-llama")
        .unwrap();
    assert_eq!(created["label"], "Local Llama");
    assert_eq!(created["key"], "••••••••");

    let (status, events) = stream(
        state,
        &token,
        "Remove local llama",
        &next_chat(),
        json!({ "admin": { "page": "models" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["success"], true);
    assert!(output["data"]["models"]
        .as_array()
        .unwrap()
        .iter()
        .all(|model| model["id"] != "local-llama"));
}

#[tokio::test]
async fn admin_chat_can_open_console_pages() {
    let llm = Arc::new(tool_then_text(
        "admin_open_page",
        json!({ "page": "Sources" }),
        "Opened sources.",
    ));
    let state = AppState::with_llm(admin_config(), llm);
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state,
        &token,
        "Open sources",
        &next_chat(),
        json!({ "admin": { "page": "dashboard" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["success"], true);
    assert_eq!(output["page"], "sources");
    assert_eq!(output["reload"], true);
    assert_eq!(output["data"]["url"], "/admin/sources");
}

#[tokio::test]
async fn admin_chat_does_not_execute_unknown_operator_tools() {
    let llm = Arc::new(tool_then_text(
        "admin_wipe_database",
        json!({}),
        "I cannot.",
    ));
    let state = AppState::with_llm(admin_config(), llm);
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state,
        &token,
        "Delete everything",
        &next_chat(),
        json!({ "admin": { "page": "dashboard" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let called = events.iter().any(|event| {
        matches!(
            event,
            StreamEvent::ToolCall { tool_name, .. } if tool_name == "admin_wipe_database"
        )
    });
    let executed = events.iter().any(|event| {
        matches!(
            event,
            StreamEvent::ToolResult { tool_name, .. } if tool_name == "admin_wipe_database"
        )
    });
    assert!(called);
    assert!(!executed);
}

#[tokio::test]
async fn admin_chat_can_spawn_dashboard_as_a_subtask() {
    let llm = Arc::new(tool_then_text(
        "spawn_tasks",
        json!({
            "tasks": [{ "type": "admin_dashboard", "label": "Stats" }]
        }),
        "Stats loaded.",
    ));
    let state = AppState::with_llm(admin_config(), llm);
    let token = admin_session_token(state.clone()).await;
    let (status, events) = stream(
        state,
        &token,
        "Collect stats",
        &next_chat(),
        json!({ "admin": { "page": "dashboard" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let output = tool_result(&events);
    assert_eq!(output["success"], true);
    assert_eq!(output["data"]["tasks"][0]["success"], true);
    assert_eq!(output["data"]["tasks"][0]["label"], "Stats");
}

#[tokio::test]
async fn admin_json_message_runs_operator_tools() {
    let llm = Arc::new(tool_then_text(
        "admin_dashboard",
        json!({}),
        "Dashboard is quiet.",
    ));
    let state = AppState::with_llm(admin_config(), llm);
    let token = admin_session_token(state.clone()).await;
    let mut headers = json_headers();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/message",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "prompt": "Show stats",
                "messages": [{ "id": "m1", "role": "user", "content": "Show stats" }],
                "chatId": next_chat(),
                "context": { "admin": { "page": "dashboard" } }
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["explanation"], "Dashboard is quiet.");
}

#[tokio::test]
async fn admin_bearer_token_is_accepted() {
    let llm = Arc::new(hello_world());
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    let token = admin_session_token(state.clone()).await;
    let mut headers = json_headers();
    headers.insert("accept", "text/event-stream".parse().unwrap());
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {token}").parse().unwrap(),
    );
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "prompt": "Hello",
                "messages": [{ "id": "m1", "role": "user", "content": "Hello" }],
                "chatId": next_chat()
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.is_empty());
    assert!(advertised(&llm).contains(&"admin_dashboard".to_string()));
}

#[tokio::test]
async fn public_chat_does_not_get_admin_tools() {
    let llm = Arc::new(tool_then_text("admin_dashboard", json!({}), "I cannot."));
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    let token = public_token(state.clone(), "visitor-public").await;
    let (status, events) = stream(
        state,
        &token,
        "Show stats",
        &next_chat(),
        json!({ "admin": { "page": "dashboard" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let executed = events.iter().any(|event| {
        matches!(
            event,
            StreamEvent::ToolResult { tool_name, .. } if tool_name == "admin_dashboard"
        )
    });
    assert!(!executed);
    let tools = advertised(&llm);
    for name in ADMIN_TOOLS {
        assert!(!tools.contains(&name.to_string()), "{name}");
    }
    assert!(tools.contains(&"web_search".to_string()));
    assert!(tools.contains(&"web_fetch".to_string()));
}

#[tokio::test]
async fn admin_chat_does_not_get_index_tools_when_a_workspace_exists() {
    let (config, root) = indexed_config();
    let llm = Arc::new(hello_world());
    let state = AppState::with_llm(config, llm.clone() as Arc<dyn LlmClient>);
    let token = admin_session_token(state.clone()).await;
    let (status, _) = stream(
        state,
        &token,
        "Search the code",
        &next_chat(),
        json!({ "admin": { "page": "sources" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tools = advertised(&llm);
    assert!(tools.contains(&"admin_open_page".to_string()));
    assert!(!tools.contains(&"search_code".to_string()));
    assert!(!tools.contains(&"read_code_file".to_string()));
    let _ = std::fs::remove_dir_all(root);
}

#[tokio::test]
async fn public_chat_gets_index_tools_when_a_workspace_exists() {
    let (config, root) = indexed_config();
    let llm = Arc::new(hello_world());
    let state = AppState::with_llm(config, llm.clone() as Arc<dyn LlmClient>);
    let token = public_token(state.clone(), "visitor-index").await;
    let (status, _) = stream(state, &token, "Search the code", &next_chat(), json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let tools = advertised(&llm);
    assert!(tools.contains(&"search_code".to_string()));
    assert!(tools.contains(&"read_code_file".to_string()));
    assert!(tools.contains(&"web_search".to_string()));
    assert!(!tools.contains(&"admin_dashboard".to_string()));
    let _ = std::fs::remove_dir_all(root);
}
