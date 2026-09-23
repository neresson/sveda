use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_llm::{LlmClient, ScriptedClient};
use sveda_protocol::HEADER_EMBED_TOKEN;
use sveda_server::{app, AppState, Config};
use tower::ServiceExt;

const DEEPWIKI_MCP: &str = "https://mcp.deepwiki.com/mcp";

fn require_public_mcp() -> bool {
    matches!(
        std::env::var("SVEDA_REQUIRE_PUBLIC_MCP")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("yes")
    )
}

fn skip_public_mcp() -> bool {
    matches!(
        std::env::var("SVEDA_SKIP_PUBLIC_MCP")
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

#[tokio::test]
async fn public_deepwiki_host_mcp_reaches_the_model() {
    if skip_public_mcp() {
        return;
    }

    let mut config = Config::test();
    config.host_api_key = Some("sveda-host-secret".into());
    config.mcp_timeout = 25;
    let llm = Arc::new(ScriptedClient::hello_world());
    let state = AppState::with_llm(config, llm.clone() as Arc<dyn LlmClient>);

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    headers.insert("x-sveda-host-key", "sveda-host-secret".parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "visitor_id": "visitor-deepwiki",
                "host_mcp_url": DEEPWIKI_MCP
            }))
            .unwrap(),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let mut stream_headers = HeaderMap::new();
    stream_headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    stream_headers.insert(header::ACCEPT, "text/event-stream".parse().unwrap());
    stream_headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());

    let listed = tokio::time::timeout(
        Duration::from_secs(30),
        send(
            state,
            "POST",
            "/sveda/stream",
            stream_headers,
            Body::from(
                serde_json::to_vec(&json!({
                    "messages": [{ "id": "m1", "role": "user", "content": "hello" }],
                    "chatId": "chat-deepwiki"
                }))
                .unwrap(),
            ),
        ),
    )
    .await;

    let (status, _, _) = match listed {
        Ok(result) => result,
        Err(_) if require_public_mcp() => {
            panic!("public MCP timed out while listing DeepWiki tools")
        }
        Err(_) => {
            eprintln!("skip public host MCP (DeepWiki timed out)");
            return;
        }
    };
    if status != StatusCode::OK {
        if require_public_mcp() {
            panic!("public host MCP stream failed: {status}");
        }
        eprintln!("skip public host MCP (stream {status})");
        return;
    }

    let instructions = llm
        .last_instructions
        .lock()
        .expect("instructions")
        .clone()
        .unwrap_or_default();
    if instructions.is_empty() && !require_public_mcp() {
        eprintln!("skip public host MCP (no instructions captured)");
        return;
    }
    assert!(
        instructions.contains("read_wiki_structure"),
        "DeepWiki tools missing from model instructions: {instructions}"
    );
    assert!(
        instructions.contains("read_wiki_contents"),
        "DeepWiki tools missing from model instructions: {instructions}"
    );
    assert!(
        instructions.contains("ask_wiki_question"),
        "DeepWiki tools missing from model instructions: {instructions}"
    );
}
