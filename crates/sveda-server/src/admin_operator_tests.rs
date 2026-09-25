use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_llm::{LlmChunk, LlmClient, ScriptedClient};
use sveda_protocol::{parse_sse_line, StreamEvent, ADMIN_VISITOR_ID, HEADER_EMBED_TOKEN};
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::token;
use crate::{app, AppState, Config};

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

async fn admin_session_token(state: AppState) -> String {
    let mut headers = HeaderMap::new();
    headers.insert("x-sveda-admin-key", "sveda-admin-secret".parse().unwrap());
    headers.insert(header::HOST, "127.0.0.1:8787".parse().unwrap());
    let (status, _, body) = send(state, "POST", "/admin/session", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
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
                "context": { "admin": { "page": "dashboard" } }
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

async fn mock_host_mcp() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/mcp/sveda"))
        .respond_with(|request: &wiremock::Request| {
            let payload: Value = serde_json::from_slice(&request.body).unwrap_or(json!({}));
            let id = payload.get("id").cloned().unwrap_or(json!(1));
            match payload.get("method").and_then(Value::as_str).unwrap_or("") {
                "initialize" => ResponseTemplate::new(200).set_body_json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "host", "version": "0.1.0" },
                        "instructions": "Host calendar tools."
                    }
                })),
                "tools/list" => ResponseTemplate::new(200).set_body_json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [{
                            "name": "get_calendar_events",
                            "description": "Get calendar events",
                            "inputSchema": { "type": "object", "properties": {} },
                            "_meta": { "domain": "calendar", "mode": "read" }
                        }]
                    }
                })),
                _ => ResponseTemplate::new(202),
            }
        })
        .mount(&server)
        .await;
    server
}

fn advertised(llm: &ScriptedClient) -> Vec<String> {
    llm.last_tools.lock().expect("tools")[0].clone()
}

#[tokio::test]
async fn reserved_visitor_without_admin_claim_does_not_get_operator_tools() {
    let llm = Arc::new(hello_world());
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    let token = token::issue(&state.config.hmac_key, ADMIN_VISITOR_ID, 120);
    let claims = token::parse(&state.config.hmac_key, &token).unwrap();
    assert_eq!(claims.visitor_id, ADMIN_VISITOR_ID);
    assert!(!claims.admin);

    let (status, _) = stream(state, &token, "Show stats", "scope-forged-admin").await;
    assert_eq!(status, StatusCode::OK);
    let tools = advertised(&llm);
    assert!(!tools.iter().any(|name| name.starts_with("admin_")));
}

#[tokio::test]
async fn admin_session_does_not_call_host_mcp_even_if_credentials_are_stored() {
    let server = mock_host_mcp().await;
    let llm = Arc::new(hello_world());
    let state = AppState::with_llm(admin_config(), llm.clone() as Arc<dyn LlmClient>);
    state.inject_host_mcp(
        ADMIN_VISITOR_ID,
        &format!("{}/mcp/sveda", server.uri()),
        "mcp-secret",
    );
    assert!(state.host_mcp(ADMIN_VISITOR_ID).is_some());

    let token = admin_session_token(state.clone()).await;
    let (status, _) = stream(state, &token, "Show stats", "scope-admin-no-host").await;
    assert_eq!(status, StatusCode::OK);

    let tools = advertised(&llm);
    assert!(tools.contains(&"admin_dashboard".to_string()));
    assert!(!tools.contains(&"get_calendar_events".to_string()));
    assert!(!tools.contains(&"web_search".to_string()));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn public_visitor_still_loads_host_mcp() {
    let server = mock_host_mcp().await;
    let llm = Arc::new(hello_world());
    let mut config = admin_config();
    config.host_api_key = Some("host-secret".into());
    let state = AppState::with_llm(config, llm.clone() as Arc<dyn LlmClient>);
    state.inject_host_mcp(
        "visitor-host",
        &format!("{}/mcp/sveda", server.uri()),
        "mcp-secret",
    );
    let token = token::issue(&state.config.hmac_key, "visitor-host", 120);
    let (status, _) = stream(state, &token, "Hello", "scope-public-host").await;
    assert_eq!(status, StatusCode::OK);
    let tools = advertised(&llm);
    assert!(tools.contains(&"get_calendar_events".to_string()));
    assert!(!tools.iter().any(|name| name.starts_with("admin_")));
    assert!(!server.received_requests().await.unwrap().is_empty());
}
