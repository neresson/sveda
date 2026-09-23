use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, HeaderMap, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_llm::{LlmChunk, LlmClient, ScriptedClient};
use sveda_protocol::{parse_sse_line, StreamEvent, HEADER_EMBED_TOKEN};
use sveda_server::{app, AppState, Config};
use tower::ServiceExt;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

async fn public_token(state: AppState, visitor_id: &str) -> String {
    let (status, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        Body::from(serde_json::to_vec(&json!({ "visitor_id": visitor_id })).unwrap()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

fn advertised(llm: &ScriptedClient) -> Vec<String> {
    llm.last_tools.lock().expect("tools")[0].clone()
}

async fn stream(
    state: AppState,
    token: &str,
    prompt: &str,
    chat_id: &str,
) -> (StatusCode, Vec<StreamEvent>) {
    let mut headers = json_headers();
    headers.insert("accept", "text/event-stream".parse().unwrap());
    headers.insert(HEADER_EMBED_TOKEN, token.parse().unwrap());
    let (status, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        Body::from(
            serde_json::to_vec(&json!({
                "prompt": prompt,
                "messages": [{ "id": "m1", "role": "user", "content": prompt }],
                "chatId": chat_id
            }))
            .unwrap(),
        ),
    )
    .await;
    let events: Vec<StreamEvent> = String::from_utf8(body)
        .unwrap()
        .lines()
        .filter_map(parse_sse_line)
        .collect();
    (status, events)
}

#[tokio::test]
async fn public_chat_can_search_via_configured_searxng() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .and(query_param("q", "sveda docs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{
                "title": "Sveda docs",
                "url": "https://sveda.dev/docs",
                "content": "Wire protocol and sidecar."
            }]
        })))
        .mount(&server)
        .await;

    let mut config = Config::test();
    config.searxng_url = server.uri();
    let llm = Arc::new(ScriptedClient::queue(vec![
        vec![
            Ok(LlmChunk::ToolCall {
                id: "call-1".into(),
                name: "web_search".into(),
                input: json!({ "query": "sveda docs" }),
            }),
            Ok(LlmChunk::Usage {
                prompt_tokens: 4,
                completion_tokens: 2,
            }),
            Ok(LlmChunk::End {
                finish_reason: "tool_calls".into(),
            }),
        ],
        vec![
            Ok(LlmChunk::TextDelta("Found the docs.".into())),
            Ok(LlmChunk::Usage {
                prompt_tokens: 6,
                completion_tokens: 3,
            }),
            Ok(LlmChunk::End {
                finish_reason: "stop".into(),
            }),
        ],
    ]));
    let state = AppState::with_llm(config, llm.clone() as Arc<dyn LlmClient>);
    let token = public_token(state.clone(), "visitor-web").await;
    let (status, events) = stream(state, &token, "Search sveda docs", "chat-web").await;
    assert_eq!(status, StatusCode::OK);
    assert!(advertised(&llm).contains(&"web_search".to_string()));
    let output = events.iter().find_map(|event| match event {
        StreamEvent::ToolResult {
            tool_name, output, ..
        } if tool_name == "web_search" => Some(output.clone()),
        _ => None,
    });
    let output = output.expect("web_search result");
    assert_eq!(output["success"], true);
    assert_eq!(output["data"]["backend"], "searxng");
    assert_eq!(
        output["data"]["results"][0]["url"],
        "https://sveda.dev/docs"
    );
    assert_eq!(output["links"][0]["url"], "https://sveda.dev/docs");
}

#[tokio::test]
async fn disabling_web_hides_internet_tools() {
    let mut config = Config::test();
    config.web_enabled = false;
    let llm = Arc::new(ScriptedClient::hello_world());
    let state = AppState::with_llm(config, llm.clone() as Arc<dyn LlmClient>);
    let token = public_token(state.clone(), "visitor-noweb").await;
    let (status, _) = stream(state, &token, "Search the web", "chat-noweb").await;
    assert_eq!(status, StatusCode::OK);
    let tools = advertised(&llm);
    assert!(!tools.contains(&"web_search".to_string()));
    assert!(!tools.contains(&"web_fetch".to_string()));
    let instructions = llm
        .last_instructions
        .lock()
        .expect("instructions")
        .clone()
        .unwrap_or_default();
    assert!(!instructions.contains("web_search"));
}

#[tokio::test]
async fn public_chat_searches_the_live_web() {
    let llm = Arc::new(ScriptedClient::queue(vec![
        vec![
            Ok(LlmChunk::ToolCall {
                id: "call-live".into(),
                name: "web_search".into(),
                input: json!({ "query": "sveda.dev official site" }),
            }),
            Ok(LlmChunk::Usage {
                prompt_tokens: 4,
                completion_tokens: 2,
            }),
            Ok(LlmChunk::End {
                finish_reason: "tool_calls".into(),
            }),
        ],
        vec![
            Ok(LlmChunk::TextDelta("https://sveda.dev".into())),
            Ok(LlmChunk::Usage {
                prompt_tokens: 8,
                completion_tokens: 4,
            }),
            Ok(LlmChunk::End {
                finish_reason: "stop".into(),
            }),
        ],
    ]));
    let state = AppState::with_llm(Config::test(), llm.clone() as Arc<dyn LlmClient>);
    let token = public_token(state.clone(), "visitor-live-web").await;
    let (status, events) = stream(state, &token, "Search sveda.dev", "chat-live-web").await;
    assert_eq!(status, StatusCode::OK);
    let output = events.iter().find_map(|event| match event {
        StreamEvent::ToolResult {
            tool_name, output, ..
        } if tool_name == "web_search" => Some(output.clone()),
        _ => None,
    });
    let output = output.expect("web_search result");
    assert_eq!(output["success"], true, "{output}");
    let hits = output["data"]["results"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        !hits.is_empty(),
        "live search returned no hits via {}",
        output["data"]["backend"]
    );
    let url = hits[0]["url"].as_str().unwrap_or_default();
    assert!(
        url.starts_with("http://") || url.starts_with("https://"),
        "first hit was not a public URL: {url}"
    );
}
