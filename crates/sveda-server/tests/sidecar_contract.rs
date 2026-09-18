use axum::body::Body;
use axum::http::{HeaderMap, HeaderName, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sveda_llm::{LlmClient, ScriptedClient};
use sveda_protocol::{
    load_sidecar_contract, parse_sse_line, SidecarContract, ACCEPT_SVEDA_STREAM, PROTOCOL_VERSION,
    SSE_DONE_LINE, STREAM_EVENTS, TOKEN_PREFIX,
};
use sveda_server::{app, AppState, Config};
use sveda_store::Checkpoint;
use tower::ServiceExt;

fn contract() -> SidecarContract {
    load_sidecar_contract(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/protocol/contracts/sidecar.v1.json"
    ))
}

fn test_state() -> AppState {
    AppState::new(Config::test())
}

fn test_state_with(config: Config) -> AppState {
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
    let response_headers = response.headers().clone();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes()
        .to_vec();
    (status, response_headers, bytes)
}

fn json_body(value: &Value) -> Body {
    Body::from(serde_json::to_vec(value).expect("json"))
}

fn json_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers
}

#[tokio::test]
async fn embed_token_matches_sidecar_contract() {
    let contract = contract();
    let (status, _, body) = send(
        test_state(),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        json_body(&json!({ "visitor_id": "visitor-contract" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    for key in &contract.embed_token.response_required {
        assert!(payload.get(key).is_some(), "missing {key}");
    }
    assert_eq!(payload["visitor_id"], "visitor-contract");
    assert!(payload["expires_in"].as_u64().unwrap() >= 60);
    assert!(payload["token"].as_str().unwrap().starts_with(TOKEN_PREFIX));
}

#[tokio::test]
async fn embed_token_requires_host_key_when_configured() {
    let mut config = Config::test();
    config.host_api_key = Some("sidecar-host-secret".into());
    let state = test_state_with(config.clone());

    let (status, _, _) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        json_body(&json!({ "visitor_id": "visitor-abc" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let mut headers = json_headers();
    headers.insert("x-sveda-host-key", "sidecar-host-secret".parse().unwrap());
    let (status, _, _) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        headers,
        json_body(&json!({
            "visitor_id": "visitor-mcp",
            "host_mcp_url": "http://127.0.0.1:8001/mcp/sveda",
            "host_mcp_token": "mcp-secret-token"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        state.host_mcp("visitor-mcp").unwrap().url,
        "http://127.0.0.1:8001/mcp/sveda"
    );

    let mut headers = json_headers();
    headers.insert(
        "authorization",
        "Bearer sidecar-host-secret".parse().unwrap(),
    );
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        headers,
        json_body(&json!({ "visitor_id": "visitor-host" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(payload["visitor_id"], "visitor-host");
}

#[tokio::test]
async fn embed_token_is_not_found_when_disabled() {
    let mut config = Config::test();
    config.embed_enabled = false;
    let (status, _, _) = send(
        test_state_with(config),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        json_body(&json!({ "visitor_id": "visitor-abc" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stream_requires_embed_token() {
    let (status, _, _) = send(
        test_state(),
        "POST",
        "/sveda/stream",
        json_headers(),
        json_body(&json!({
            "messages": [{ "id": "m1", "role": "user", "content": "Say hello" }],
            "chatId": "chat-1"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn stream_hello_world_matches_sidecar_contract() {
    let contract = contract();
    let state = test_state();
    let (status, _, token_body) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        json_body(&json!({ "visitor_id": "visitor-stream" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = serde_json::from_slice::<Value>(&token_body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let mut headers = json_headers();
    headers.insert("accept", ACCEPT_SVEDA_STREAM.parse().unwrap());
    headers.insert("x-sveda-embed-token", token.parse().unwrap());
    let (status, response_headers, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        json_body(&json!({
            "messages": [{ "id": "m1", "role": "user", "content": "Say hello" }],
            "chatId": "chat-contract-stream"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let content_type = response_headers
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.starts_with(&contract.headers.stream_response["Content-Type"]));
    assert_eq!(
        response_headers
            .get(HeaderName::from_static("x-sveda-protocol-version"))
            .unwrap()
            .to_str()
            .unwrap(),
        PROTOCOL_VERSION
    );
    assert!(response_headers
        .get("cache-control")
        .unwrap()
        .to_str()
        .unwrap()
        .contains(&contract.headers.stream_response["Cache-Control"]));
    assert_eq!(
        response_headers
            .get(HeaderName::from_static("x-accel-buffering"))
            .unwrap()
            .to_str()
            .unwrap(),
        contract.headers.stream_response["X-Accel-Buffering"]
    );

    let text = String::from_utf8(body).unwrap();
    assert!(text.contains(SSE_DONE_LINE));

    let mut types = Vec::new();
    for line in text.lines() {
        if let Some(event) = parse_sse_line(line) {
            types.push(event.event_type().to_string());
        }
    }
    assert!(types.contains(&"message.start".to_string()));
    assert!(types.contains(&"text.delta".to_string()));
    assert!(types.contains(&"message.end".to_string()));
    for event_type in &types {
        assert!(
            STREAM_EVENTS.contains(&event_type.as_str()),
            "unexpected event {event_type}"
        );
        assert!(contract.stream_events.iter().any(|item| item == event_type));
    }
}

#[tokio::test]
async fn unknown_model_returns_422() {
    let state = test_state();
    let (status, _, token_body) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        json_headers(),
        json_body(&json!({ "visitor_id": "visitor-unknown-model" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = serde_json::from_slice::<Value>(&token_body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let mut headers = json_headers();
    headers.insert("accept", ACCEPT_SVEDA_STREAM.parse().unwrap());
    headers.insert("x-sveda-embed-token", token.parse().unwrap());
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/stream",
        headers,
        json_body(&json!({
            "prompt": "Say hello",
            "model": "no-such-model"
        })),
    )
    .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    assert!(payload["message"]
        .as_str()
        .unwrap()
        .contains("Unknown Sveda model"));
}

async fn issue_token(state: AppState, visitor_id: &str) -> String {
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/embed/token",
        json_headers(),
        json_body(&json!({ "visitor_id": visitor_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string()
}

fn auth_headers(token: &str) -> HeaderMap {
    let mut headers = json_headers();
    headers.insert("x-sveda-embed-token", token.parse().unwrap());
    headers
}

fn auth_accept_headers(token: &str) -> HeaderMap {
    let mut headers = auth_headers(token);
    headers.insert("accept", ACCEPT_SVEDA_STREAM.parse().unwrap());
    headers
}

#[tokio::test]
async fn message_and_histories_match_sidecar_contract() {
    let contract = contract();
    let state = test_state();
    let token = issue_token(state.clone(), "visitor-histories").await;
    let headers = auth_headers(&token);

    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/sveda/message",
        headers.clone(),
        json_body(&json!({
            "messages": [{ "id": "m1", "role": "user", "content": "Say hello" }],
            "chatId": "chat-contract-message"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&body).unwrap();
    for key in ["explanation", "tokens_used", "chat_id"] {
        assert!(payload.get(key).is_some(), "missing {key}");
    }
    assert_eq!(payload["explanation"], "Hello from Sveda");
    assert_eq!(payload["chat_id"], "chat-contract-message");

    let (status, _, body) = send(
        state.clone(),
        "GET",
        "/sveda/chat-histories",
        headers.clone(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let list: Value = serde_json::from_slice(&body).unwrap();
    let histories = list["histories"].as_array().expect("histories");
    assert_eq!(histories.len(), 1);
    for key in &contract.histories.summary_required {
        assert!(histories[0].get(key).is_some(), "missing summary {key}");
    }
    assert_eq!(histories[0]["chatId"], "chat-contract-message");

    let (status, _, body) = send(
        state.clone(),
        "GET",
        "/sveda/chat-histories/chat-contract-message",
        headers.clone(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let detail: Value = serde_json::from_slice(&body).unwrap();
    let history = &detail["history"];
    for key in &contract.histories.detail_required {
        assert!(history.get(key).is_some(), "missing detail {key}");
    }
    assert!(history["messages"].as_array().unwrap().len() >= 2);

    let (status, _, body) = send(
        state.clone(),
        "PATCH",
        "/sveda/chat-histories/chat-contract-message",
        headers.clone(),
        json_body(&json!({ "title": "Contract title" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mutation: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(mutation["success"], true);

    let (status, _, body) = send(
        state.clone(),
        "GET",
        "/sveda/chat-histories/chat-contract-message",
        headers.clone(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let renamed: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(renamed["history"]["title"], "Contract title");

    let (status, _, body) = send(
        state.clone(),
        "DELETE",
        "/sveda/chat-histories/chat-contract-message",
        headers.clone(),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let deleted: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(deleted["success"], true);

    let (status, _, _) = send(
        state,
        "GET",
        "/sveda/chat-histories/chat-contract-message",
        headers,
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn histories_are_scoped_to_embed_visitor() {
    let state = test_state();
    let token_a = issue_token(state.clone(), "visitor-a").await;
    let token_b = issue_token(state.clone(), "visitor-b").await;

    let (status, _, _) = send(
        state.clone(),
        "POST",
        "/sveda/message",
        auth_headers(&token_a),
        json_body(&json!({
            "messages": [{ "id": "m1", "role": "user", "content": "Secret A" }],
            "chatId": "chat-secret"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _, body) = send(
        state.clone(),
        "GET",
        "/sveda/chat-histories",
        auth_headers(&token_b),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let list: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(list["histories"].as_array().unwrap().len(), 0);

    let (status, _, _) = send(
        state,
        "GET",
        "/sveda/chat-histories/chat-secret",
        auth_headers(&token_b),
        Body::empty(),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn stream_emits_chat_title_for_placeholder() {
    let state = test_state();
    let token = issue_token(state.clone(), "visitor-title").await;
    let (status, _, body) = send(
        state,
        "POST",
        "/sveda/stream",
        auth_accept_headers(&token),
        json_body(&json!({
            "messages": [{ "id": "m1", "role": "user", "content": "Explain ocean tides" }],
            "chatId": "chat-title-1"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let text = String::from_utf8(body).unwrap();
    assert!(text.contains("\"type\":\"chat.title\""));
    assert!(text.contains("Explain ocean tides") || text.contains("\"title\":"));
}

#[tokio::test]
async fn document_extract_matches_sidecar_contract() {
    let contract = contract();
    let state = test_state();
    let token = issue_token(state.clone(), "visitor-extract").await;
    let boundary = "----SvedaBoundary";
    let mut body = String::new();
    body.push_str(&format!("--{boundary}\r\n"));
    body.push_str("Content-Disposition: form-data; name=\"files\"; filename=\"note.txt\"\r\n");
    body.push_str("Content-Type: text/plain\r\n\r\n");
    body.push_str("hello from contract\n");
    body.push_str(&format!("\r\n--{boundary}--\r\n"));

    let mut headers = HeaderMap::new();
    headers.insert(
        "content-type",
        format!("multipart/form-data; boundary={boundary}")
            .parse()
            .unwrap(),
    );
    headers.insert("x-sveda-embed-token", token.parse().unwrap());

    let (status, _, bytes) = send(
        state,
        "POST",
        "/sveda/documents/extract",
        headers,
        Body::from(body),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    let items = payload["items"].as_array().expect("items");
    assert_eq!(items.len(), 1);
    for key in &contract.documents_extract.item_required {
        assert!(items[0].get(key).is_some(), "missing item {key}");
    }
    assert_eq!(items[0]["ok"], true);
    assert_eq!(items[0]["filename"], "note.txt");
    assert_eq!(items[0]["text"], "hello from contract\n");
}

#[tokio::test]
async fn compaction_summary_trims_llm_history() {
    let mut config = Config::test();
    config.compaction_keep_tail = 1;
    let llm = std::sync::Arc::new(ScriptedClient::hello_world());
    let state = AppState::with_llm(config, llm.clone() as std::sync::Arc<dyn LlmClient>);
    let token = issue_token(state.clone(), "visitor-compact").await;
    state
        .store()
        .checkpoint(Checkpoint {
            visitor_id: "visitor-compact".into(),
            chat_id: "chat-compact".into(),
            title: "Compacted".into(),
            preview: "old".into(),
            messages: Vec::new(),
            conversation_history: (0..5)
                .map(|index| json!({"role": "user", "content": format!("old-{index}")}))
                .collect(),
            tokens_used: 0,
        })
        .await
        .expect("checkpoint");
    state
        .store()
        .set_summary("visitor-compact", "chat-compact", "Frozen summary".into())
        .await
        .expect("summary");

    let (status, _, _) = send(
        state,
        "POST",
        "/sveda/stream",
        auth_accept_headers(&token),
        json_body(&json!({
            "messages": [{ "id": "m1", "role": "user", "content": "What next?" }],
            "chatId": "chat-compact"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let counts = llm.last_message_counts.lock().expect("counts").clone();
    assert_eq!(counts.first().copied(), Some(2));
    let instructions = llm.last_instructions.lock().expect("instructions").clone();
    assert!(instructions.unwrap_or_default().contains("Frozen summary"));
}

#[tokio::test]
async fn host_mcp_instructions_and_tool_names_reach_the_model() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/mcp/sveda"))
        .respond_with(|request: &wiremock::Request| {
            let payload: Value = serde_json::from_slice(&request.body).unwrap_or(json!({}));
            let id = payload.get("id").cloned().unwrap_or(json!(1));
            match payload.get("method").and_then(Value::as_str).unwrap_or("") {
                "initialize" => ResponseTemplate::new(200).set_body_json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2025-11-25",
                        "capabilities": { "tools": { "listChanged": false } },
                        "serverInfo": { "name": "playground", "version": "0.1.0" },
                        "instructions": "Playground feed tools."
                    }
                })),
                "tools/list" => ResponseTemplate::new(200).set_body_json(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [{
                            "name": "create_post",
                            "description": "Create a post",
                            "inputSchema": { "type": "object", "properties": {} },
                            "_meta": { "domain": "posts", "mode": "write" }
                        }]
                    }
                })),
                _ => ResponseTemplate::new(202),
            }
        })
        .mount(&server)
        .await;

    let mut config = Config::test();
    config.host_api_key = Some("sidecar-host-secret".into());
    let llm = std::sync::Arc::new(ScriptedClient::hello_world());
    let state = AppState::with_llm(config, llm.clone() as std::sync::Arc<dyn LlmClient>);

    let mut token_headers = json_headers();
    token_headers.insert("x-sveda-host-key", "sidecar-host-secret".parse().unwrap());
    let (status, _, body) = send(
        state.clone(),
        "POST",
        "/sveda/embed/token",
        token_headers,
        json_body(&json!({
            "visitor_id": "visitor-mcp-instructions",
            "host_mcp_url": format!("{}/mcp/sveda", server.uri()),
            "host_mcp_token": "mcp-secret"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = serde_json::from_slice::<Value>(&body).unwrap()["token"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _, _) = send(
        state,
        "POST",
        "/sveda/stream",
        auth_accept_headers(&token),
        json_body(&json!({
            "messages": [{ "id": "m1", "role": "user", "content": "создай тестовый пост" }],
            "chatId": "chat-mcp-instructions"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let instructions = llm
        .last_instructions
        .lock()
        .expect("instructions")
        .clone()
        .unwrap_or_default();
    assert!(
        instructions.contains("Playground feed tools."),
        "instructions missing server text: {instructions}"
    );
    assert!(
        instructions.contains("create_post"),
        "instructions missing tool name: {instructions}"
    );
    assert!(
        instructions.contains("MUST call the matching tool"),
        "instructions missing usage directive: {instructions}"
    );
}

#[tokio::test]
async fn embed_widget_assets_allow_cross_origin_module_load() {
    let mut config = Config::test();
    config.cors_origins = vec!["http://localhost:8001".into()];
    let mut headers = HeaderMap::new();
    headers.insert("origin", "http://127.0.0.1:8788".parse().unwrap());

    let (status, response_headers, body) = send(
        test_state_with(config),
        "GET",
        "/build/sveda/sveda-chat.js",
        headers,
        Body::empty(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(!body.is_empty());
    assert_eq!(
        response_headers.get("access-control-allow-origin").unwrap(),
        "*"
    );
    assert_eq!(
        response_headers.get("cache-control").unwrap(),
        "public, max-age=0, must-revalidate"
    );
    assert_eq!(
        response_headers.get("cloudflare-cdn-cache-control").unwrap(),
        "no-cache"
    );
}

#[tokio::test]
async fn cors_preflight_allows_contract_headers() {
    let mut config = Config::test();
    config.cors_origins = vec!["http://localhost:8001".into()];
    let mut headers = HeaderMap::new();
    headers.insert("origin", "http://localhost:8001".parse().unwrap());
    headers.insert("access-control-request-method", "POST".parse().unwrap());
    headers.insert(
        "access-control-request-headers",
        "content-type,x-sveda-embed-token,x-sveda-protocol"
            .parse()
            .unwrap(),
    );

    let (status, response_headers, _) = send(
        test_state_with(config),
        "OPTIONS",
        "/sveda/stream",
        headers,
        Body::empty(),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response_headers.get("access-control-allow-origin").unwrap(),
        "http://localhost:8001"
    );
    let allow_headers = response_headers
        .get("access-control-allow-headers")
        .unwrap()
        .to_str()
        .unwrap()
        .to_ascii_lowercase();
    assert!(allow_headers.contains("x-sveda-embed-token"));
    assert!(allow_headers.contains("x-sveda-protocol"));
}
