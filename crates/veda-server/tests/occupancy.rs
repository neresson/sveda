use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;
use veda_protocol::{HEADER_EMBED_TOKEN, TOKEN_PREFIX};
use veda_server::{app, AppState, Config};

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
        "/veda/embed/token",
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
        "/veda/embed/token",
        json_headers(),
        Body::from(serde_json::to_vec(&json!({ "visitor_id": "v1" })).unwrap()),
    )
    .await;
    assert_eq!(first, StatusCode::OK);
    let (second, headers, body) = send(
        state,
        "POST",
        "/veda/embed/token",
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
        "/veda/stream",
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
        "/veda/stream",
        headers,
        Body::from(serde_json::to_vec(&json!({ "prompt": "Hi" })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8(body).unwrap();
    assert!(text.contains("Hello from Veda"));
}
