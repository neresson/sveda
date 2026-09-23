use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
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

#[tokio::test]
async fn health_reports_rust_runtime() {
    let (status, _, body) = send(
        AppState::new(Config::test()),
        "GET",
        "/sveda/health",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["runtime"], "rust");
    assert_eq!(
        payload["version"],
        format!("v{}", env!("CARGO_PKG_VERSION"))
    );
}

#[tokio::test]
async fn ready_reports_ok_in_memory_mode() {
    let (status, _, body) = send(
        AppState::new(Config::test()),
        "GET",
        "/sveda/ready",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["ok"], true);
    assert_eq!(payload["runtime"], "rust");
}

#[tokio::test]
async fn setup_page_renders_when_admin_key_is_missing() {
    let (status, _, body) = send(
        AppState::new(Config::test()),
        "GET",
        "/admin",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("id=\"sveda-admin\""));
    assert!(html.contains("\"page\":\"setup\""));
    assert!(html.contains("/admin/setup"));
}

#[tokio::test]
async fn login_page_renders_when_unauthenticated() {
    let (status, _, body) = send(
        admin_state(),
        "GET",
        "/admin",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("id=\"sveda-admin\""));
    assert!(html.contains("\"page\":\"login\""));
    assert!(html.contains("/admin/login"));
}

#[tokio::test]
async fn login_rejects_wrong_key() {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded".parse().unwrap(),
    );
    let (status, response_headers, _) = send(
        admin_state(),
        "POST",
        "/admin/login",
        headers,
        Body::from("key=wrong"),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let location = response_headers
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(location.contains("/admin"));
    assert!(response_headers.get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn dashboard_page_renders_after_login() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("\"page\":\"dashboard\""));
    assert!(html.contains("/admin/models"));
    assert!(html.contains("/admin/mcp"));
    assert!(html.contains("/admin/appearance"));
    assert!(html.contains("/admin/prompts"));
    assert!(html.contains("/admin/security"));
    assert!(html.contains("/admin/settings"));
}

#[tokio::test]
async fn dashboard_and_usage_include_recorded_requests() {
    let state = admin_state();
    state
        .usage()
        .record(sveda_store::UsageEvent {
            visitor_id: "visitor-1".into(),
            chat_id: "chat-1".into(),
            model: "deepseek-v4-flash-responses".into(),
            status: "completed".into(),
            prompt_tokens: 8,
            completion_tokens: 2,
            tokens_used: 10,
        })
        .await
        .unwrap();
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
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["page"], "dashboard");
    assert_eq!(payload["stats"]["requests"], 1);
    assert_eq!(payload["stats"]["tokens_used"], 10);
    assert_eq!(payload["stats"]["prompt_tokens"], 8);
    assert_eq!(payload["stats"]["completion_tokens"], 2);
    assert_eq!(payload["stats"]["series"].as_array().unwrap().len(), 14);

    let (status, _, body) = send(state, "GET", "/admin/usage", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["page"], "usage");
    assert_eq!(payload["usage"]["requests"]["total"], 1);
    assert_eq!(payload["usage"]["by_model"][0]["tokens_used"], 10);
    assert_eq!(
        payload["usage"]["by_model"][0]["model_label"],
        "DeepSeek V4 Flash (Responses)"
    );
}

#[tokio::test]
async fn runtime_page_renders_after_login() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin/runtime", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("\"page\":\"runtime\""));
}

#[tokio::test]
async fn models_page_renders_after_login() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin/models", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("\"page\":\"models\""));
}

#[tokio::test]
async fn mcp_and_appearance_pages_render_after_login() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    for page in ["mcp", "appearance", "usage", "sources", "security"] {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, cookie.parse().unwrap());
        let (status, _, body) = send(
            state.clone(),
            "GET",
            &format!("/admin/{page}"),
            headers,
            Body::empty(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{page}");
        let html = String::from_utf8(body).unwrap();
        assert!(html.contains(&format!("\"page\":\"{page}\"")), "{page}");
    }
}

#[tokio::test]
async fn admin_spa_accepts_json() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::ACCEPT, "application/json".parse().unwrap());
    let (status, response_headers, body) =
        send(state, "GET", "/admin/mcp", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(response_headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .contains("application/json"));
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["page"], "mcp");
    assert_eq!(payload["urls"]["appearance"], "/admin/appearance");
    assert_eq!(payload["urls"]["security"], "/admin/security");
    assert_eq!(payload["chat"]["sessionUrl"], "/admin/session");
    assert_eq!(payload["chat"]["prefix"], "sveda");
}

#[tokio::test]
async fn unknown_admin_section_is_not_found() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin/unknown", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("id=\"sveda-site\""));
    assert!(html.contains("Section not found"));
}

#[tokio::test]
async fn session_can_update_settings() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    let (status, _, body) = send(
        state,
        "POST",
        "/admin/settings",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "welcome_message": "Hello from rust runtime"
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["welcome_message"], "Hello from rust runtime");
}

#[tokio::test]
async fn setup_creates_admin_key() {
    let state = AppState::new(Config::test());
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/x-www-form-urlencoded".parse().unwrap(),
    );
    let (status, response_headers, _) = send(
        state.clone(),
        "POST",
        "/admin/setup",
        headers,
        Body::from(
            "key=sveda-admin-secret-from-setup&key_confirmation=sveda-admin-secret-from-setup",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let cookie = cookie_from(&response_headers);
    assert!(cookie.starts_with("sveda_admin="));
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, body) = send(state, "GET", "/admin", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("\"page\":\"dashboard\""));
}

#[tokio::test]
async fn embed_page_requires_token() {
    let (status, _, _) = send(
        AppState::new(Config::test()),
        "GET",
        "/sveda/embed",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn embed_page_rejects_invalid_token() {
    let (status, _, _) = send(
        AppState::new(Config::test()),
        "GET",
        "/sveda/embed?token=invalid",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn embed_page_renders_host_driven_iframe_shell() {
    let state = AppState::new(Config::test());
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        headers,
        Body::from(r#"{"visitor_id":"php-playground"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    let token = payload["token"].as_str().expect("token");

    let (status, _, body) = send(
        state,
        "GET",
        &format!("/sveda/embed?token={token}"),
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("id=\"sveda-embed\""));
    assert!(html.contains("\"hideLauncher\":true"));
    assert!(html.contains("/build/sveda/embed.js"));
    assert!(html.contains(
        "#sveda-embed{margin:0;width:100%;height:100%;background:transparent;overflow:hidden}"
    ));
}

#[tokio::test]
async fn admin_session_mints_embed_token_after_login() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::HOST, "127.0.0.1:8787".parse().unwrap());
    let (status, _, body) = send(state, "POST", "/admin/session", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["origin"], "http://127.0.0.1:8787");
    assert!(payload["token"]
        .as_str()
        .unwrap_or_default()
        .starts_with("sveda_embed_"));
    assert_eq!(payload["expires_in"], 3600);
}

#[tokio::test]
async fn admin_session_requires_cookie() {
    let (status, _, _) = send(
        admin_state(),
        "POST",
        "/admin/session",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_session_is_not_found_when_embed_disabled() {
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    config.embed_enabled = false;
    let state = AppState::new(config);
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, _) = send(state, "POST", "/admin/session", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
