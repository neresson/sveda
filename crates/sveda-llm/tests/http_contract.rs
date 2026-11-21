use futures_util::StreamExt;
use serde_json::json;
use sveda_llm::{
    ChatMessage, HttpClient, LlmChunk, LlmClient, ModelSpec, Protocol, StepRequest, StoredToolCall,
    ToolSpec,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn raw_upstream(response: String) -> ModelSpec {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 4096];
        loop {
            let n = socket.read(&mut buffer).await.unwrap();
            assert_ne!(n, 0);
            request.extend_from_slice(&buffer[..n]);
            if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                let size: usize = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length: "))
                    .unwrap()
                    .parse()
                    .unwrap();
                if request.len() >= end + 4 + size {
                    break;
                }
            }
        }
        socket.write_all(response.as_bytes()).await.unwrap();
        socket.shutdown().await.unwrap();
    });
    fixture_model(format!("http://{address}"))
}

fn fixture_model(url: String) -> ModelSpec {
    ModelSpec {
        id: "deepseek-v4-flash-responses".into(),
        label: "Flash".into(),
        protocol: Protocol::Responses,
        api_model: "deepseek-v4-flash".into(),
        url,
        key: "fixture-key".into(),
        aliases: Vec::new(),
        reasoning_effort: None,
    }
}

#[tokio::test]
async fn truncated_chunked_approval_followup_reports_safe_transport_error() {
    let model = raw_upstream(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n100\r\n: partial"
            .into(),
    )
    .await;
    let chunks: Vec<_> = HttpClient::from_timeout(5)
        .stream_step(&model, tool_followup(true))
        .collect()
        .await;
    assert_eq!(chunks.len(), 1);
    assert_eq!(
        chunks[0].as_ref().unwrap_err().to_string(),
        "The connection to the model provider ended while reading its response."
    );
}

#[tokio::test]
async fn terminal_event_prevents_a_later_framing_failure_from_discarding_the_response() {
    let body = responses_sse();
    let model = raw_upstream(format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{body}\r\n100\r\n: partial",
        body.len()
    ))
    .await;
    let chunks: Vec<_> = HttpClient::from_timeout(5)
        .stream_step(&model, tool_followup(true))
        .collect()
        .await;
    assert!(chunks.iter().all(Result::is_ok));
    assert!(
        matches!(chunks.last(), Some(Ok(LlmChunk::End { finish_reason })) if finish_reason == "stop")
    );
}

#[tokio::test]
async fn clean_eof_without_completion_is_reported_even_after_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            "data: {\"type\":\"response.output_text.delta\",\"delta\":\"partial\"}\n\n",
            "text/event-stream",
        ))
        .mount(&server)
        .await;
    let model = fixture_model(server.uri());
    let chunks: Vec<_> = HttpClient::from_timeout(5)
        .stream_step(&model, tool_followup(true))
        .collect()
        .await;
    assert_eq!(chunks.len(), 2);
    assert!(matches!(&chunks[0], Ok(LlmChunk::TextDelta(text)) if text == "partial"));
    assert_eq!(
        chunks[1].as_ref().unwrap_err().to_string(),
        "The model provider ended the stream before completing the response."
    );
}

#[tokio::test]
async fn incompatible_responses_are_reported_without_exposing_upstream_payloads() {
    for (status, media, encoding, body, expected) in [
        (
            400,
            "application/json",
            "identity",
            r#"{"error":{"message":"fixture-key"}}"#,
            "The model provider returned HTTP 400.",
        ),
        (
            200,
            "application/json",
            "identity",
            r#"{"message":"fixture-key"}"#,
            "The model provider returned an unexpected response format.",
        ),
        (
            200,
            "text/event-stream",
            "gzip",
            "invalid compressed bytes",
            "The model provider returned an unexpected response format.",
        ),
        (
            200,
            "text/event-stream",
            "identity",
            "data: {fixture-key}\n\n",
            "The model provider sent an unreadable event.",
        ),
    ] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/responses"))
            .and(header("accept", "text/event-stream"))
            .and(header("accept-encoding", "identity"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("content-encoding", encoding)
                    .set_body_raw(body, media),
            )
            .mount(&server)
            .await;
        let model = fixture_model(server.uri());
        let chunks: Vec<_> = HttpClient::from_timeout(5)
            .stream_step(&model, tool_followup(true))
            .collect()
            .await;
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].as_ref().unwrap_err().to_string(), expected);
    }
}

fn responses_sse() -> String {
    "event: response.output_text.delta\n\
data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello from responses\"}\n\
\n\
event: response.completed\n\
data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"usage\":{\"input_tokens\":12,\"output_tokens\":8}}}\n\
\n"
        .to_string()
}

fn anthropic_sse() -> String {
    "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":12}}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello from anthropic\"}}\n\
\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":8}}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
\n"
        .to_string()
}

fn request(thinking: bool) -> StepRequest {
    StepRequest {
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Say hello".into(),
            ..Default::default()
        }],
        instructions: Some("You are Sveda.".into()),
        thinking,
        tools: Vec::new(),
    }
}

async fn collect_text(llm: &HttpClient, model: &ModelSpec, thinking: bool) -> Vec<String> {
    let mut stream = llm.stream_step(model, request(thinking));
    let mut texts = Vec::new();
    while let Some(item) = stream.next().await {
        if let Ok(LlmChunk::TextDelta(text)) = item {
            texts.push(text);
        }
    }
    texts
}

#[tokio::test]
async fn responses_thinking_on_posts_effort_high() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(body_partial_json(json!({
            "model": "deepseek-v4-flash",
            "store": false,
            "reasoning": { "effort": "high" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(responses_sse(), "text/event-stream"))
        .mount(&server)
        .await;

    let model = ModelSpec {
        id: "deepseek-v4-flash-responses".into(),
        label: "flash".into(),
        protocol: Protocol::Responses,
        api_model: "deepseek-v4-flash".into(),
        url: server.uri(),
        key: "test-deepseek-key".into(),
        aliases: Vec::new(),
        reasoning_effort: None,
    };
    let texts = collect_text(&HttpClient::from_timeout(5), &model, true).await;
    assert_eq!(texts, vec!["Hello from responses".to_string()]);
}

#[tokio::test]
async fn responses_thinking_off_posts_effort_none() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(body_partial_json(json!({
            "reasoning": { "effort": "none" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(responses_sse(), "text/event-stream"))
        .mount(&server)
        .await;

    let model = ModelSpec {
        id: "flash".into(),
        label: "flash".into(),
        protocol: Protocol::Responses,
        api_model: "deepseek-v4-flash".into(),
        url: server.uri(),
        key: "k".into(),
        aliases: Vec::new(),
        reasoning_effort: None,
    };
    let texts = collect_text(&HttpClient::from_timeout(5), &model, false).await;
    assert_eq!(texts, vec!["Hello from responses".to_string()]);
}

#[tokio::test]
async fn anthropic_thinking_on_posts_enabled() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(body_partial_json(json!({
            "model": "deepseek-v4-flash",
            "thinking": { "type": "enabled", "budget_tokens": 8192 }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(anthropic_sse(), "text/event-stream"))
        .mount(&server)
        .await;

    let model = ModelSpec {
        id: "anthropic".into(),
        label: "anthropic".into(),
        protocol: Protocol::Anthropic,
        api_model: "deepseek-v4-flash".into(),
        url: server.uri(),
        key: "k".into(),
        aliases: Vec::new(),
        reasoning_effort: None,
    };
    let texts = collect_text(&HttpClient::from_timeout(5), &model, true).await;
    assert_eq!(texts, vec!["Hello from anthropic".to_string()]);
}

#[tokio::test]
async fn http_429_is_failoverable() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate_limit"))
        .mount(&server)
        .await;

    let model = ModelSpec {
        id: "flash".into(),
        label: "flash".into(),
        protocol: Protocol::Responses,
        api_model: "deepseek-v4-flash".into(),
        url: server.uri(),
        key: "k".into(),
        aliases: Vec::new(),
        reasoning_effort: None,
    };
    let mut stream = HttpClient::from_timeout(5).stream_step(&model, request(true));
    let err = stream.next().await.unwrap().unwrap_err();
    assert!(err.is_failoverable());
}

fn tool_followup(thinking: bool) -> StepRequest {
    StepRequest {
        messages: vec![
            ChatMessage {
                role: "user".into(),
                content: "что ты умеешь?".into(),
                ..Default::default()
            },
            ChatMessage {
                role: "assistant".into(),
                reasoning: "check tools".into(),
                tool_calls: vec![StoredToolCall {
                    id: "call-1".into(),
                    name: "search_agent_tools".into(),
                    arguments: "{}".into(),
                }],
                ..Default::default()
            },
            ChatMessage {
                role: "tool".into(),
                content: "ok".into(),
                tool_call_id: Some("call-1".into()),
                ..Default::default()
            },
        ],
        instructions: None,
        thinking,
        tools: vec![ToolSpec {
            name: "search_agent_tools".into(),
            description: "Search tools".into(),
            parameters: json!({ "type": "object", "properties": {} }),
        }],
    }
}

#[tokio::test]
async fn deepseek_thinking_posts_reasoning_text_with_function_call() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(body_partial_json(json!({
            "input": [
                { "role": "user", "content": "что ты умеешь?" },
                {
                    "type": "reasoning",
                    "content": [{ "type": "reasoning_text", "text": "check tools" }]
                },
                { "role": "assistant", "content": "" },
                {
                    "type": "function_call",
                    "call_id": "call-1",
                    "name": "search_agent_tools",
                    "arguments": "{}"
                },
                {
                    "type": "function_call_output",
                    "call_id": "call-1",
                    "output": "ok"
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(responses_sse(), "text/event-stream"))
        .mount(&server)
        .await;

    let model = ModelSpec {
        id: "deepseek-v4-flash-responses".into(),
        label: "flash".into(),
        protocol: Protocol::Responses,
        api_model: "deepseek-v4-flash".into(),
        url: server.uri(),
        key: "k".into(),
        aliases: Vec::new(),
        reasoning_effort: None,
    };
    let mut stream = HttpClient::from_timeout(5).stream_step(&model, tool_followup(true));
    let mut texts = Vec::new();
    while let Some(item) = stream.next().await {
        if let Ok(LlmChunk::TextDelta(text)) = item {
            texts.push(text);
        }
    }
    assert_eq!(texts, vec!["Hello from responses".to_string()]);
}

#[tokio::test]
async fn openai_thinking_does_not_post_plaintext_reasoning() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/responses"))
        .and(body_partial_json(json!({
            "input": [
                { "role": "user", "content": "что ты умеешь?" },
                {
                    "type": "function_call",
                    "call_id": "call-1",
                    "name": "search_agent_tools",
                    "arguments": "{}"
                },
                {
                    "type": "function_call_output",
                    "call_id": "call-1",
                    "output": "ok"
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_raw(responses_sse(), "text/event-stream"))
        .mount(&server)
        .await;

    let model = ModelSpec {
        id: "gpt-4o".into(),
        label: "GPT".into(),
        protocol: Protocol::Responses,
        api_model: "gpt-4o".into(),
        url: server.uri(),
        key: "k".into(),
        aliases: Vec::new(),
        reasoning_effort: None,
    };
    let mut stream = HttpClient::from_timeout(5).stream_step(&model, tool_followup(true));
    let mut texts = Vec::new();
    while let Some(item) = stream.next().await {
        if let Ok(LlmChunk::TextDelta(text)) = item {
            texts.push(text);
        }
    }
    assert_eq!(texts, vec!["Hello from responses".to_string()]);
}

#[tokio::test]
async fn openai_embeddings_post_model_input_and_dimensions() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/embeddings"))
        .and(body_partial_json(json!({
            "model": "text-embedding-3-small",
            "input": ["hello code"],
            "dimensions": 1536
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [{
                "index": 0,
                "embedding": vec![0.0; 1536]
            }]
        })))
        .mount(&server)
        .await;

    let spec = sveda_llm::EmbeddingSpec {
        id: "openai-small".into(),
        label: "small".into(),
        protocol: sveda_llm::EmbeddingProtocol::OpenAi,
        api_model: "text-embedding-3-small".into(),
        url: server.uri(),
        key: "sk-test".into(),
        dimensions: 1536,
        usd_per_million: 0.02,
    };
    let vectors = HttpClient::from_timeout(5)
        .embed(&spec, &["hello code".into()])
        .await
        .expect("embed");
    assert_eq!(vectors.len(), 1);
    assert_eq!(vectors[0].len(), 1536);
}
