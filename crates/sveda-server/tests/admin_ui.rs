use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
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

#[tokio::test]
async fn home_does_not_serve_the_admin_panel() {
    let (status, body) = send(admin_state(), "GET", "/", HeaderMap::new(), Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let html = String::from_utf8(body).unwrap();
    assert!(html.contains("sveda.yaml"));
    assert!(!html.contains("/admin/models"));
}

#[tokio::test]
async fn html_admin_routes_are_gone() {
    let (status, _) = send(
        admin_state(),
        "GET",
        "/admin",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn settings_and_session_use_the_admin_key() {
    let state = admin_state();
    let (status, _) = send(
        state.clone(),
        "GET",
        "/admin/settings",
        HeaderMap::new(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let mut headers = HeaderMap::new();
    headers.insert("x-sveda-admin-key", "sveda-admin-secret".parse().unwrap());
    headers.insert(header::HOST, "127.0.0.1:8787".parse().unwrap());
    let (status, body) = send(
        state.clone(),
        "GET",
        "/admin/settings",
        headers.clone(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let settings: Value = serde_json::from_slice(&body).unwrap();
    assert!(settings.get("models").is_some());

    let (status, body) = send(state, "POST", "/admin/session", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let session: Value = serde_json::from_slice(&body).unwrap();
    assert!(session["token"]
        .as_str()
        .unwrap()
        .starts_with("sveda_embed_"));
}
