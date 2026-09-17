use futures_util::StreamExt;
use serde_json::json;
use sveda_llm::{
    ChatMessage, HttpClient, LlmChunk, LlmClient, ModelSpec, Protocol, StepRequest, StoredToolCall,
    ToolSpec,
};
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(responses_sse()),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(responses_sse()),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(anthropic_sse()),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(responses_sse()),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(responses_sse()),
        )
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
