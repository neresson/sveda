use std::env;
use std::fs;
use std::path::PathBuf;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_protocol::{parse_sse_line, StreamEvent, HEADER_EMBED_TOKEN};
use sveda_server::{app, AppState, Config};
use tower::ServiceExt;

fn live_key() -> Option<String> {
    for name in ["DEEPSEEK_API_KEY", "SVEDA_DEEPSEEK_API_KEY"] {
        if let Ok(value) = env::var(name) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "../../apps/runtime/.env",
        "../../../sveda/apps/runtime/.env",
    ] {
        let Ok(contents) = fs::read_to_string(manifest.join(relative)) else {
            continue;
        };
        for raw in contents.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line = line.strip_prefix("export ").unwrap_or(line).trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() != "DEEPSEEK_API_KEY" && key.trim() != "SVEDA_DEEPSEEK_API_KEY" {
                continue;
            }
            let mut value = value.trim().to_string();
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn require_live_llm() -> bool {
    matches!(
        env::var("SVEDA_REQUIRE_LIVE_LLM")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("yes")
    )
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

fn stream_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert(header::ACCEPT, "text/event-stream".parse().unwrap());
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    headers
}

async fn mint(state: AppState, visitor: &str) -> String {
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    let (status, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        headers,
        Body::from(serde_json::to_vec(&json!({ "visitor_id": visitor })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

fn sse_text(body: &[u8]) -> String {
    sse_events(body)
        .iter()
        .filter_map(|event| match event {
            StreamEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect()
}

fn sse_events(body: &[u8]) -> Vec<StreamEvent> {
    String::from_utf8_lossy(body)
        .lines()
        .filter_map(parse_sse_line)
        .collect()
}

fn sse_errors(events: &[StreamEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            StreamEvent::Error { message, .. } => Some(message.clone()),
            _ => None,
        })
        .collect()
}

fn skip_or_assert_contains(text: &str, errors: &[String], needle: &str, case_insensitive: bool) {
    if !errors.is_empty() {
        if require_live_llm() {
            panic!("live DeepSeek failed: {errors:?}");
        }
        eprintln!("skip live server chat: {errors:?}");
        return;
    }
    let haystack = if case_insensitive {
        text.to_uppercase()
    } else {
        text.to_string()
    };
    let needle = if case_insensitive {
        needle.to_uppercase()
    } else {
        needle.to_string()
    };
    assert!(
        haystack.contains(&needle),
        "unexpected DeepSeek reply text={text:?}"
    );
}

async fn live_state() -> Option<AppState> {
    let key = live_key()?;
    if env::var("DEEPSEEK_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_none()
    {
        env::set_var("DEEPSEEK_API_KEY", &key);
    }
    let mut config = Config::test();
    config.admin_api_key = Some("sveda-admin-secret".into());
    config.stream_timeout = 90;
    Some(AppState::live(config).await)
}

#[tokio::test]
async fn live_server_stream_answers_without_thinking() {
    let Some(state) = live_state().await else {
        eprintln!("skip live server chat: DEEPSEEK_API_KEY is not set");
        return;
    };
    let token = mint(state.clone(), "live-plain").await;
    let (status, body) = send(
        state,
        "POST",
        "/sveda/stream",
        stream_headers(&token),
        Body::from(
            serde_json::to_vec(&json!({
                "messages": [{ "id": "m1", "role": "user", "content": "Reply with exactly the word PONG and nothing else." }],
                "chatId": "live-plain",
                "options": { "thinking": false }
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let events = sse_events(&body);
    let errors = sse_errors(&events);
    let text = sse_text(&body);
    skip_or_assert_contains(&text, &errors, "PONG", true);
}

#[tokio::test]
async fn live_server_stream_answers_with_thinking() {
    let Some(state) = live_state().await else {
        eprintln!("skip live server chat: DEEPSEEK_API_KEY is not set");
        return;
    };
    let token = mint(state.clone(), "live-think").await;
    let (status, body) = send(
        state.clone(),
        "POST",
        "/sveda/stream",
        stream_headers(&token),
        Body::from(
            serde_json::to_vec(&json!({
                "messages": [{ "id": "m1", "role": "user", "content": "What is 8 + 5? Reply with the number only." }],
                "chatId": "live-think",
                "options": { "thinking": true }
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let events = sse_events(&body);
    let errors = sse_errors(&events);
    let text = sse_text(&body);
    if !errors.is_empty() {
        if require_live_llm() {
            panic!("live DeepSeek failed: {errors:?}");
        }
        eprintln!("skip live server chat: {errors:?}");
        return;
    }
    skip_or_assert_contains(&text, &errors, "13", false);

    let mut headers = HeaderMap::new();
    headers.insert("x-sveda-admin-key", "sveda-admin-secret".parse().unwrap());
    headers.insert(header::ACCEPT, "application/json".parse().unwrap());
    let (status, body) = send(state, "GET", "/admin/usage", headers, Body::empty()).await;
    assert_eq!(status, StatusCode::OK);
    let dashboard: Value = serde_json::from_slice(&body).unwrap();
    assert!(
        dashboard["stats"]["requests"].as_u64().unwrap() >= 1,
        "dashboard did not record the live turn: {dashboard}"
    );
    assert!(dashboard["stats"]["tokens_used"].as_u64().unwrap() >= 1);
}
