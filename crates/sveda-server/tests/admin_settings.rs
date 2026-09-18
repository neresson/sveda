use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_protocol::{HEADER_ADMIN_KEY, HEADER_EMBED_TOKEN, TOKEN_PREFIX};
use sveda_server::{app, AppState, Config};
use tower::ServiceExt;

fn admin_state() -> AppState {
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    AppState::new(config)
}

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
    headers.insert("content-type", "application/json".parse().unwrap());
    headers
}

fn admin_headers() -> HeaderMap {
    let mut headers = json_headers();
    headers.insert(HEADER_ADMIN_KEY, "sveda-admin-secret".parse().unwrap());
    headers
}

#[tokio::test]
async fn admin_settings_are_not_found_when_admin_key_is_unconfigured() {
    let (status, _) = send(
        AppState::new(Config::test()),
        "GET",
        "/admin/settings",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_settings_require_admin_key() {
    let (status, _) = send(
        admin_state(),
        "GET",
        "/admin/settings",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn embed_host_key_cannot_read_admin_settings() {
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    config.host_api_key = Some("sidecar-host-secret".into());
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        "Bearer sidecar-host-secret".parse().unwrap(),
    );
    let (status, _) = send(
        AppState::new(config),
        "GET",
        "/admin/settings",
        headers,
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reads_settings_with_admin_key_header() {
    let (status, body) = send(
        admin_state(),
        "GET",
        "/admin/settings",
        admin_headers(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["default_model"], "deepseek-v4-flash-responses");
    assert_eq!(payload["max_steps"], 30);
    assert!(payload.get("welcome_message").is_some());
    assert!(payload.get("system_prompt").is_some());
    assert!(payload.get("models").is_some());
    assert!(payload.get("compaction").is_some());
    assert!(payload.get("cors").is_some());
    assert!(payload.get("failover").is_some());
    assert!(
        payload["deepseek"]["key"].as_str().unwrap() == ""
            || payload["deepseek"]["key"] == "••••••••"
    );
    let model_key = payload["models"][0]["key"].as_str().unwrap();
    assert!(model_key.is_empty() || model_key == "••••••••");
}

#[tokio::test]
async fn reads_settings_with_bearer_admin_key() {
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        "Bearer sveda-admin-secret".parse().unwrap(),
    );
    let (status, _) = send(
        admin_state(),
        "GET",
        "/admin/settings",
        headers,
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn updates_settings_and_masks_secrets() {
    let state = admin_state();
    let (status, _) = send(
        state.clone(),
        "PUT",
        "/admin/settings",
        admin_headers(),
        Body::from(
            serde_json::to_vec(&json!({
                "default_model": "custom-responses",
                "failover": ["deepseek-v4-flash-anthropic"],
                "deepseek": { "key": "••••••••" },
                "models": [{
                    "id": "custom-responses",
                    "label": "Custom Responses",
                    "protocol": "responses",
                    "api_model": "gpt-test",
                    "url": "https://example.test/v1",
                    "key": "custom-secret-key",
                    "thinking": false,
                    "vision": false
                }],
                "max_steps": 18,
                "compaction": {
                    "enabled": false,
                    "min_messages": 12,
                    "keep_tail_messages": 6
                },
                "cors": { "allowed_origins": ["https://app.example.test"] },
                "welcome_message": "Welcome to Acme copilot",
                "system_prompt": "Always answer in Russian."
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        state,
        "GET",
        "/admin/settings",
        admin_headers(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["default_model"], "custom-responses");
    assert_eq!(payload["welcome_message"], "Welcome to Acme copilot");
    assert_eq!(payload["system_prompt"], "Always answer in Russian.");
    assert_eq!(payload["max_steps"], 18);
    assert_eq!(payload["compaction"]["enabled"], false);
    assert_eq!(payload["compaction"]["min_messages"], 12);
    assert_eq!(
        payload["cors"]["allowed_origins"][0],
        "https://app.example.test"
    );
    assert_eq!(payload["deepseek"]["key"], "");
    assert_eq!(payload["models"][0]["key"], "••••••••");
    assert_eq!(payload["models"][0]["id"], "custom-responses");
}

#[tokio::test]
async fn post_can_add_and_update_model_without_replacing_key() {
    let state = admin_state();
    let (status, body) = send(
        state.clone(),
        "POST",
        "/admin/settings",
        admin_headers(),
        Body::from(
            serde_json::to_vec(&json!({
                "models": [{
                    "id": "custom-flash",
                    "label": "Custom Flash",
                    "protocol": "responses",
                    "api_model": "flash",
                    "url": "https://api.example.test",
                    "key": "model-secret",
                    "thinking": true,
                    "vision": false,
                    "aliases": ["flash"]
                }]
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["models"][0]["id"], "custom-flash");
    assert_eq!(payload["models"][0]["key"], "••••••••");

    let (status, body) = send(
        state,
        "POST",
        "/admin/settings",
        admin_headers(),
        Body::from(
            serde_json::to_vec(&json!({
                "models": [{
                    "id": "custom-flash",
                    "label": "Custom Flash Updated",
                    "protocol": "anthropic",
                    "api_model": "flash-2",
                    "url": "https://api.example.test/v2",
                    "key": "",
                    "thinking": false,
                    "vision": true,
                    "aliases": ["flash"]
                }]
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["models"][0]["label"], "Custom Flash Updated");
    assert_eq!(payload["models"][0]["protocol"], "anthropic");
    assert_eq!(payload["models"][0]["key"], "••••••••");
}

#[tokio::test]
async fn public_embed_config_omits_secrets() {
    let state = admin_state();
    let (status, _) = send(
        state.clone(),
        "PUT",
        "/admin/settings",
        admin_headers(),
        Body::from(
            serde_json::to_vec(&json!({
                "default_model": "deepseek-v4-flash-responses",
                "models": [{
                    "id": "deepseek-v4-flash-responses",
                    "label": "DeepSeek Flash",
                    "protocol": "responses",
                    "api_model": "deepseek-v4-flash",
                    "url": "https://api.deepseek.com",
                    "key": "must-not-leak",
                    "thinking": true
                }],
                "welcome_message": "Hi from Sveda",
                "system_prompt": "Secret host instructions",
                "deepseek": { "key": "must-not-leak-shared" }
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(serde_json::to_vec(&json!({ "visitor_id": "visitor-config" })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(token.starts_with(TOKEN_PREFIX));

    let mut headers = HeaderMap::new();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, body) = send(state, "GET", "/sveda/embed/config", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let text = String::from_utf8(body).unwrap();
    assert_eq!(payload["welcome_message"], "Hi from Sveda");
    assert_eq!(payload["default_model"], "deepseek-v4-flash-responses");
    assert_eq!(payload["model"], "deepseek-v4-flash-responses");
    assert_eq!(payload["models"][0]["id"], "deepseek-v4-flash-responses");
    assert_eq!(payload["models"][0]["protocol"], "responses");
    assert_eq!(payload["models"][0]["supportsThinking"], true);
    assert!(payload.get("deepseek").is_none());
    assert!(payload.get("system_prompt").is_none());
    assert!(!text.contains("must-not-leak"));
}

#[tokio::test]
async fn public_embed_config_requires_embed_token() {
    let (status, _) = send(
        admin_state(),
        "GET",
        "/sveda/embed/config",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn updated_model_is_usable_on_stream() {
    let state = admin_state();
    let (status, _) = send(
        state.clone(),
        "POST",
        "/admin/settings",
        admin_headers(),
        Body::from(
            serde_json::to_vec(&json!({
                "models": [{
                    "id": "custom-flash",
                    "label": "Custom Flash",
                    "protocol": "responses",
                    "api_model": "flash",
                    "url": "https://api.example.test",
                    "key": "model-secret",
                    "thinking": true
                }]
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, body) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(serde_json::to_vec(&json!({ "visitor_id": "visitor-model" })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let mut headers = json_headers();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "prompt": "Hi",
                "model": "custom-flash"
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8(body).unwrap();
    assert!(text.contains("Hello from Sveda"));
}
