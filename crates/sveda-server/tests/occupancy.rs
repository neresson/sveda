use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_protocol::{HEADER_EMBED_TOKEN, TOKEN_PREFIX};
use sveda_server::{app, AppState, Config};
use tower::ServiceExt;

fn json_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers
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

async fn token_for(state: AppState, visitor: &str) -> String {
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

#[tokio::test]
async fn embed_token_throttle_returns_429() {
    let mut config = Config::test();
    config.embed_throttle_max = 1;
    config.embed_throttle_window_secs = 60;
    let state = AppState::new(config);
    let (first, _, _) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(serde_json::to_vec(&json!({ "visitor_id": "v1" })).unwrap()),
    )
    .await;
    assert_eq!(first, StatusCode::OK);
    let (second, headers, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(serde_json::to_vec(&json!({ "visitor_id": "v2" })).unwrap()),
    )
    .await;
    assert_eq!(second, StatusCode::TOO_MANY_REQUESTS);
    assert!(headers.get("retry-after").is_some());
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["message"], "Too Many Attempts.");
}

#[tokio::test]
async fn occupied_stream_slot_returns_429() {
    let mut config = Config::test();
    config.occupancy_global = 1;
    let state = AppState::new(config);
    let _lease = state.occupancy.acquire("held").expect("held");
    let token = token_for(state.clone(), "visitor-occ").await;
    assert!(token.starts_with(TOKEN_PREFIX));
    let mut headers = json_headers();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(serde_json::to_vec(&json!({ "prompt": "Hi" })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["message"], "Too many active streams.");
}

#[tokio::test]
async fn stream_succeeds_when_occupancy_slot_is_free() {
    let mut config = Config::test();
    config.occupancy_global = 1;
    let state = AppState::new(config);
    let token = token_for(state.clone(), "visitor-free").await;
    let mut headers = json_headers();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(serde_json::to_vec(&json!({ "prompt": "Hi" })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8(body).unwrap();
    assert!(text.contains("Hello from Sveda"));
}

#[tokio::test]
async fn stream_throttle_returns_429_per_visitor() {
    let mut config = Config::test();
    config.stream_throttle_max = 1;
    config.stream_throttle_window_secs = 60;
    let state = AppState::new(config);
    let token = token_for(state.clone(), "visitor-stream-rl").await;
    let mut headers = json_headers();
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (first, _, _) = send(
        state.clone(),
        "POST",
        "/sveda/stream",
        headers.clone(),
        Body::from(serde_json::to_vec(&json!({ "prompt": "Hi" })).unwrap()),
    )
    .await;
    assert_eq!(first, StatusCode::OK);
    let (second, retry_headers, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(serde_json::to_vec(&json!({ "prompt": "Hi" })).unwrap()),
    )
    .await;
    assert_eq!(second, StatusCode::TOO_MANY_REQUESTS);
    assert!(retry_headers.get("retry-after").is_some());
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["message"], "Too Many Attempts.");
}

#[tokio::test]
async fn ip_throttle_returns_429_when_header_configured() {
    let mut config = Config::test();
    config.client_ip_header = "CF-Connecting-IP".into();
    config.ip_throttle_max = 1;
    config.ip_throttle_window_secs = 60;
    let state = AppState::new(config);
    let token_a = token_for(state.clone(), "visitor-ip-a").await;
    let token_b = token_for(state.clone(), "visitor-ip-b").await;
    let mut headers_a = json_headers();
    headers_a.insert(HEADER_EMBED_TOKEN, token_a.parse().unwrap());
    headers_a.insert("cf-connecting-ip", "203.0.113.9".parse().unwrap());
    let (first, _, _) = send(
        state.clone(),
        "POST",
        "/sveda/stream",
        headers_a,
        Body::from(serde_json::to_vec(&json!({ "prompt": "Hi" })).unwrap()),
    )
    .await;
    assert_eq!(first, StatusCode::OK);
    let mut headers_b = json_headers();
    headers_b.insert(HEADER_EMBED_TOKEN, token_b.parse().unwrap());
    headers_b.insert("cf-connecting-ip", "203.0.113.9".parse().unwrap());
    let (second, _, _) = send(
        state,
        "POST",
        "/sveda/stream",
        headers_b,
        Body::from(serde_json::to_vec(&json!({ "prompt": "Hi" })).unwrap()),
    )
    .await;
    assert_eq!(second, StatusCode::TOO_MANY_REQUESTS);
}
