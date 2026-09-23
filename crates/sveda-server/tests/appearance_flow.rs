use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_protocol::{HEADER_EMBED_TOKEN, TOKEN_PREFIX};
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

async fn mint_embed_token(state: AppState) -> String {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        headers,
        Body::from(serde_json::to_vec(&json!({ "visitor_id": "appearance-e2e" })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    payload["token"].as_str().expect("token").to_string()
}

#[tokio::test]
async fn public_embed_config_defaults_to_empty_appearance() {
    let state = admin_state();
    let token = mint_embed_token(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "GET",
        "/sveda/embed/config",
        headers,
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["appearance"], json!({}));

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
    assert!(html.contains("\"appearance\":{}"));
    assert!(html.contains("id=\"sveda-embed\""));
}

#[tokio::test]
async fn saved_admin_appearance_is_published_on_embed_config() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/admin/settings",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "appearance": {
                    "preset": "lms",
                    "radius": "8px",
                    "launcher": { "label": "ADMIN" }
                }
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let saved: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(saved["appearance"]["preset"], "lms");
    assert_eq!(saved["appearance"]["radius"], "8px");

    let token = mint_embed_token(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "GET",
        "/sveda/embed/config",
        headers,
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["appearance"]["preset"], "lms");
    assert_eq!(payload["appearance"]["launcher"]["label"], "ADMIN");

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
    assert!(html.contains("\"preset\":\"lms\""));
    assert!(html.contains("id=\"sveda-embed\""));
}

#[tokio::test]
async fn admin_session_token_can_read_embed_config() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    headers.insert(header::HOST, "127.0.0.1:8787".parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/admin/session",
        headers,
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let session: Value = serde_json::from_slice(&body).unwrap();
    let token = session["token"].as_str().expect("token");
    assert!(token.starts_with(TOKEN_PREFIX));
    assert_eq!(session["origin"], "http://127.0.0.1:8787");

    let mut headers = HeaderMap::new();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, _, _) = send(state, "GET", "/sveda/embed/config", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn authenticated_admin_pages_keep_chat_session_url() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    for page in [
        "dashboard",
        "appearance",
        "models",
        "prompts",
        "runtime",
        "security",
        "mcp",
        "usage",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, cookie.parse().unwrap());
        headers.insert(header::ACCEPT, "application/json".parse().unwrap());
        let (status, _, body) = send(
            state.clone(),
            "GET",
            &format!("/admin/{page}"),
            headers,
            Body::empty(),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{page}");
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["page"], page);
        assert_eq!(payload["chat"]["sessionUrl"], "/admin/session");
        assert_eq!(payload["chat"]["prefix"], "sveda");
    }
}
