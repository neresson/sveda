use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use veda_server::{app, AppState, Config};

fn admin_state() -> AppState {
    let mut config = Config::test();
    config.admin_api_key = Some("veda-admin-secret".into());
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
        "/veda/admin/login",
        headers,
        Body::from("key=veda-admin-secret"),
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
        "/veda/health",
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
        "/veda/admin",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("id=\"veda-admin\""));
    assert!(html.contains("\"page\":\"setup\""));
    assert!(html.contains("/veda/admin/setup"));
}

#[tokio::test]
async fn login_page_renders_when_unauthenticated() {
    let (status, _, body) = send(
        admin_state(),
        "GET",
        "/veda/admin",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("id=\"veda-admin\""));
    assert!(html.contains("\"page\":\"login\""));
    assert!(html.contains("/veda/admin/login"));
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
        "/veda/admin/login",
        headers,
        Body::from("key=wrong"),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let location = response_headers
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    assert!(location.contains("/veda/admin"));
    assert!(response_headers.get(header::SET_COOKIE).is_none());
}

#[tokio::test]
async fn dashboard_page_renders_after_login() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, body) = send(state, "GET", "/veda/admin", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("\"page\":\"dashboard\""));
    assert!(html.contains("/veda/admin/models"));
    assert!(html.contains("/veda/admin/mcp"));
    assert!(html.contains("/veda/admin/appearance"));
    assert!(html.contains("/veda/admin/prompts"));
    assert!(html.contains("/veda/admin/settings"));
}

#[tokio::test]
async fn runtime_page_renders_after_login() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, body) = send(state, "GET", "/veda/admin/runtime", headers, Body::empty()).await;
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
    let (status, _, body) = send(state, "GET", "/veda/admin/models", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("\"page\":\"models\""));
}

#[tokio::test]
async fn mcp_and_appearance_pages_render_after_login() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    for page in ["mcp", "appearance", "usage", "sources"] {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, cookie.parse().unwrap());
        let (status, _, body) = send(
            state.clone(),
            "GET",
            &format!("/veda/admin/{page}"),
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
        send(state, "GET", "/veda/admin/mcp", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    assert!(response_headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .contains("application/json"));
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["page"], "mcp");
    assert_eq!(payload["urls"]["appearance"], "/veda/admin/appearance");
}

#[tokio::test]
async fn unknown_admin_section_is_not_found() {
    let state = admin_state();
    let cookie = login_cookie(state.clone()).await;
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, _) = send(state, "GET", "/veda/admin/unknown", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
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
        "/veda/admin/settings",
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
        "/veda/admin/setup",
        headers,
        Body::from(
            "key=veda-admin-secret-from-setup&key_confirmation=veda-admin-secret-from-setup",
        ),
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let cookie = cookie_from(&response_headers);
    assert!(cookie.starts_with("veda_admin="));
    let mut headers = HeaderMap::new();
    headers.insert(header::COOKIE, cookie.parse().unwrap());
    let (status, _, body) = send(state, "GET", "/veda/admin", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("\"page\":\"dashboard\""));
}
