use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_llm::{LlmClient, ScriptedClient};
use sveda_protocol::{HEADER_ADMIN_KEY, HEADER_EMBED_TOKEN};
use sveda_server::{app, AppState, Config};
use tower::ServiceExt;

async fn send(
    state: AppState,
    method: &str,
    uri: &str,
    headers: HeaderMap,
    body: Body,
) -> (StatusCode, Vec<u8>) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers.iter() {
        builder = builder.header(name, value);
    }
    let request = builder.body(body).expect("request");
    let response = app(state).oneshot(request).await.expect("response");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    (status, bytes)
}

fn json_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    headers
}

fn admin_headers() -> HeaderMap {
    let mut headers = json_headers();
    headers.insert(HEADER_ADMIN_KEY, "sveda-admin-secret".parse().unwrap());
    headers
}

fn test_state() -> AppState {
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    AppState::new(config)
}

fn test_state_with_llm(llm: Arc<ScriptedClient>) -> AppState {
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    AppState::with_llm(config, llm as Arc<dyn LlmClient>)
}

fn advertised(llm: &ScriptedClient) -> Vec<String> {
    llm.last_tools.lock().expect("tools")[0].clone()
}

async fn seed_reader_policy(state: AppState) {
    let (status, _) = send(
        state,
        "POST",
        "/admin/settings",
        admin_headers(),
        Body::from(
            json!({
                "policies": {
                    "reader": {
                        "web": false,
                        "code": false,
                        "mcp": { "allow": ["echo"], "max_mode": "read" },
                        "client": { "allow": ["allowed_tool"] }
                    },
                    "writer": {
                        "web": true,
                        "code": true,
                        "mcp": { "allow": ["*"], "max_mode": "write" },
                        "client": { "allow": ["*"] }
                    }
                }
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

async fn mint_with_policy(state: AppState, visitor: &str, policy: &str) -> String {
    let (status, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(
            json!({
                "visitor_id": visitor,
                "policy": policy
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).expect("json");
    payload["token"].as_str().expect("token").to_string()
}

async fn mint_with_policy_and_grants(
    state: AppState,
    visitor: &str,
    policy: &str,
    grants: Value,
) -> String {
    let (status, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(
            json!({
                "visitor_id": visitor,
                "policy": policy,
                "grants": grants
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).expect("json");
    payload["token"].as_str().expect("token").to_string()
}

async fn mint_plain(state: AppState, visitor: &str) -> String {
    let (status, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(json!({ "visitor_id": visitor }).to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).expect("json");
    payload["token"].as_str().expect("token").to_string()
}

#[tokio::test]
async fn mint_rejects_unknown_policy() {
    let state = test_state();
    let (status, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(json!({ "visitor_id": "v1", "policy": "missing" }).to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let payload: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["message"], "unknown policy");
}

#[tokio::test]
async fn mint_rejects_invalid_policy_name() {
    let state = test_state();
    let (status, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(json!({ "visitor_id": "v1", "policy": "bad name!" }).to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let payload: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["message"], "policy name is invalid");
}

#[tokio::test]
async fn embed_config_includes_restricted_policy_capabilities() {
    let state = test_state();
    seed_reader_policy(state.clone()).await;
    let token = mint_with_policy(state.clone(), "visitor-policy", "reader").await;

    let mut headers = HeaderMap::new();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, body) = send(state, "GET", "/sveda/embed/config", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).expect("json");
    let caps = &payload["capabilities"];
    assert_eq!(caps["restricted"], true);
    assert_eq!(caps["web"], false);
    assert_eq!(caps["client"]["allow"][0], "allowed_tool");
    assert_eq!(caps["mcp"]["allow"][0], "echo");
    assert_eq!(caps["mcp"]["max_mode"], "read");
}

#[tokio::test]
async fn embed_config_without_policy_is_unrestricted() {
    let state = test_state();
    seed_reader_policy(state.clone()).await;
    let token = mint_plain(state.clone(), "visitor-open").await;

    let mut headers = HeaderMap::new();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, body) = send(state, "GET", "/sveda/embed/config", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["capabilities"]["restricted"], false);
    assert!(payload["capabilities"].get("web").is_none());
}

#[tokio::test]
async fn grants_cannot_widen_named_policy_web() {
    let state = test_state();
    seed_reader_policy(state.clone()).await;
    let token = mint_with_policy_and_grants(
        state.clone(),
        "visitor-widen",
        "reader",
        json!({ "web": true, "code": true }),
    )
    .await;

    let mut headers = HeaderMap::new();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, body) = send(state, "GET", "/sveda/embed/config", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let caps = serde_json::from_slice::<Value>(&body).expect("json")["capabilities"].clone();
    assert_eq!(caps["restricted"], true);
    assert_eq!(caps["web"], false);
    assert_eq!(caps["code"], false);
}

#[tokio::test]
async fn grants_can_tighten_named_policy_web() {
    let state = test_state();
    seed_reader_policy(state.clone()).await;
    let token = mint_with_policy_and_grants(
        state.clone(),
        "visitor-tighten",
        "writer",
        json!({ "web": false }),
    )
    .await;

    let mut headers = HeaderMap::new();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, body) = send(state, "GET", "/sveda/embed/config", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let caps = serde_json::from_slice::<Value>(&body).expect("json")["capabilities"].clone();
    assert_eq!(caps["restricted"], true);
    assert_eq!(caps["web"], false);
    assert_eq!(caps["code"], true);
}

#[tokio::test]
async fn policy_stream_accepts_restricted_client_tools() {
    let state = test_state();
    seed_reader_policy(state.clone()).await;
    let token = mint_with_policy(state.clone(), "visitor-client", "reader").await;

    let mut headers = json_headers();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    headers.insert(
        header::ACCEPT,
        "application/vnd.sveda.stream+json".parse().unwrap(),
    );

    let (status, _) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(
            json!({
                "prompt": "hello",
                "clientTools": [
                    { "name": "allowed_tool", "description": "ok", "inputSchema": { "type": "object" } },
                    { "name": "blocked_tool", "description": "no", "inputSchema": { "type": "object" } }
                ]
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn policy_stream_strips_blocked_client_tools_from_llm_advertisement() {
    let llm = Arc::new(ScriptedClient::hello_world());
    let state = test_state_with_llm(llm.clone());
    seed_reader_policy(state.clone()).await;
    let token = mint_with_policy(state.clone(), "visitor-client-filter", "reader").await;

    let mut headers = json_headers();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    headers.insert(header::ACCEPT, "text/event-stream".parse().unwrap());

    let (status, _) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(
            json!({
                "prompt": "hello",
                "chatId": "chat-policy-client",
                "clientTools": [
                    { "name": "allowed_tool", "description": "ok", "inputSchema": { "type": "object" } },
                    { "name": "blocked_tool", "description": "no", "inputSchema": { "type": "object" } }
                ]
            })
            .to_string(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tools = advertised(&llm);
    assert!(tools.contains(&"allowed_tool".to_string()));
    assert!(!tools.contains(&"blocked_tool".to_string()));
}
