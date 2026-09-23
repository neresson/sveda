use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_llm::{LlmChunk, LlmClient, ScriptedClient};
use sveda_protocol::{parse_sse_line, StreamEvent, HEADER_EMBED_TOKEN};
use sveda_server::{app, AppState, Config};
use tower::ServiceExt;

fn admin_config() -> Config {
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    config
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

async fn mint_token(state: AppState, visitor: &str) -> String {
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(serde_json::to_vec(&json!({ "visitor_id": visitor })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

fn stream_headers(token: &str) -> HeaderMap {
    let mut headers = json_headers();
    headers.insert(header::ACCEPT, "text/event-stream".parse().unwrap());
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    headers
}

fn parse_events(body: &[u8]) -> Vec<StreamEvent> {
    String::from_utf8_lossy(body)
        .lines()
        .filter_map(parse_sse_line)
        .collect()
}

fn event_types(events: &[StreamEvent]) -> Vec<&'static str> {
    events.iter().map(StreamEvent::event_type).collect()
}

fn thinking_client() -> Arc<ScriptedClient> {
    Arc::new(ScriptedClient::queue(vec![vec![
        Ok(LlmChunk::ReasoningDelta("count first".into())),
        Ok(LlmChunk::TextDelta("Hello from Sveda".into())),
        Ok(LlmChunk::Usage {
            prompt_tokens: 8,
            completion_tokens: 4,
        }),
        Ok(LlmChunk::End {
            finish_reason: "stop".into(),
        }),
    ]]))
}

#[tokio::test]
async fn stream_defaults_to_thinking_and_records_usage() {
    let llm = thinking_client();
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    let token = mint_token(state.clone(), "visitor-think").await;
    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/sveda/stream",
        stream_headers(&token),
        Body::from(
            serde_json::to_vec(&json!({
                "messages": [{ "id": "m1", "role": "user", "content": "Say hello" }],
                "chatId": "chat-think"
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(llm.last_thinking(), Some(true));

    let events = parse_events(&body);
    let types = event_types(&events);
    assert!(types.contains(&"message.start"));
    assert!(types.contains(&"reasoning.delta"));
    assert!(types.contains(&"text.delta"));
    assert!(types.contains(&"message.end"));
    let text: String = events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Hello from Sveda");

    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::ACCEPT, "application/json".parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "GET",
        "/admin",
        headers.clone(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let dashboard: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(dashboard["stats"]["requests"], 1);
    assert_eq!(dashboard["stats"]["completed"], 1);
    assert_eq!(dashboard["stats"]["tokens_used"], 12);
    assert_eq!(dashboard["stats"]["prompt_tokens"], 8);
    assert_eq!(dashboard["stats"]["completion_tokens"], 4);
    assert_eq!(dashboard["stats"]["conversations"], 1);
    assert_eq!(dashboard["stats"]["users"], 1);

    let (status, _, body) = send(state, "GET", "/admin/usage", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let usage: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(usage["usage"]["requests"]["total"], 1);
    assert_eq!(usage["usage"]["requests"]["data"][0]["tokens_used"], 12);
    assert_eq!(
        usage["usage"]["by_model"][0]["model"],
        "deepseek-v4-flash-responses"
    );
}

#[tokio::test]
async fn stream_can_disable_thinking() {
    let llm = thinking_client();
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    let token = mint_token(state.clone(), "visitor-no-think").await;
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/stream",
        stream_headers(&token),
        Body::from(
            serde_json::to_vec(&json!({
                "messages": [{ "id": "m1", "role": "user", "content": "Say hello" }],
                "chatId": "chat-no-think",
                "options": { "thinking": false }
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(llm.last_thinking(), Some(false));
    let types = event_types(&parse_events(&body));
    assert!(types.contains(&"text.delta"));
}

#[tokio::test]
async fn json_message_returns_assistant_text_and_counts_on_dashboard() {
    let state = AppState::with_llm(
        admin_config(),
        Arc::new(ScriptedClient::hello_world()) as Arc<dyn LlmClient>,
    );
    let token = mint_token(state.clone(), "visitor-json").await;
    let mut headers = json_headers();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/sveda/message",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "prompt": "Hi",
                "chatId": "chat-json"
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["explanation"], "Hello from Sveda");
    assert_eq!(payload["chat_id"], "chat-json");

    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::ACCEPT, "application/json".parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let dashboard: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(dashboard["stats"]["requests"], 1);
    assert!(dashboard["stats"]["tokens_used"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn admin_mcp_catalog_roundtrip() {
    let state = AppState::new(admin_config());
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    let catalog = json!({
        "mcpServers": {
            "deepwiki": {
                "url": "https://mcp.deepwiki.com/mcp"
            }
        }
    });
    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/admin/settings",
        headers.clone(),
        Body::from(serde_json::to_vec(&json!({ "mcp": catalog })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let saved: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(
        saved["mcp"]["mcpServers"]["deepwiki"]["url"],
        "https://mcp.deepwiki.com/mcp"
    );

    headers.remove(header::CONTENT_TYPE);
    headers.insert(header::ACCEPT, "application/json".parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin/mcp", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let page: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(page["page"], "mcp");
    assert_eq!(
        page["settings"]["mcp"]["mcpServers"]["deepwiki"]["url"],
        "https://mcp.deepwiki.com/mcp"
    );
}

#[tokio::test]
async fn stream_explicit_thinking_true_reaches_the_model() {
    let llm = thinking_client();
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    let token = mint_token(state.clone(), "visitor-think-on").await;
    let (status, _, _) = send(
        state,
        "POST",
        "/sveda/stream",
        stream_headers(&token),
        Body::from(
            serde_json::to_vec(&json!({
                "messages": [{ "id": "m1", "role": "user", "content": "Say hello" }],
                "chatId": "chat-think-on",
                "options": { "thinking": true }
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(llm.last_thinking(), Some(true));
}

#[tokio::test]
async fn mint_rejects_host_mcp_token_without_url() {
    let (status, _, body) = send(
        AppState::new(admin_config()),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(
            serde_json::to_vec(&json!({
                "visitor_id": "visitor-mcp-token-only",
                "host_mcp_token": "secret"
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["message"], "host_mcp_token requires host_mcp_url");
}

#[tokio::test]
async fn mint_stores_public_host_mcp_without_a_token() {
    let mut config = admin_config();
    config.host_api_key = Some("sveda-host-secret".into());
    let state = AppState::new(config);
    let mut headers = json_headers();
    headers.insert("x-sveda-host-key", "sveda-host-secret".parse().unwrap());
    let (status, _, _) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "visitor_id": "visitor-public-mcp",
                "host_mcp_url": "https://mcp.deepwiki.com/mcp"
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let creds = state.host_mcp("visitor-public-mcp").expect("stored mcp");
    assert_eq!(creds.url, "https://mcp.deepwiki.com/mcp");
    assert_eq!(creds.token, "");
}
