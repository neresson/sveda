use serde_json::{json, Value};
use sveda_mcp::{HostMcpClient, McpCallContext, McpCredentials};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

struct McpResponder;

impl Respond for McpResponder {
    fn respond(&self, request: &Request) -> ResponseTemplate {
        mcp_reply(request)
    }
}

fn mcp_reply(request: &Request) -> ResponseTemplate {
    let payload: Value = serde_json::from_slice(&request.body).unwrap_or(json!({}));
    let method = payload.get("method").and_then(Value::as_str).unwrap_or("");
    let id = payload.get("id").cloned().unwrap_or(json!(1));
    match method {
        "initialize" => ResponseTemplate::new(200)
            .insert_header("content-type", "application/json")
            .insert_header("mcp-session-id", "sess-test")
            .set_body_json(json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "lms", "version": "0.1.0" },
                    "instructions": "Host tools for the current user."
                }
            })),
        "notifications/initialized" => ResponseTemplate::new(202),
        "tools/list" => ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [{
                    "name": "get_calendar_events",
                    "title": "get_calendar_events",
                    "description": "Get calendar events",
                    "inputSchema": {
                        "type": "object",
                        "properties": { "limit": { "type": "integer" } }
                    },
                    "_meta": { "domain": "calendar", "mode": "read" }
                }]
            }
        })),
        "tools/call" => ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": "{\"success\":true,\"data\":{\"events\":[]}}" }],
                "isError": false
            }
        })),
        _ => ResponseTemplate::new(202),
    }
}

#[tokio::test]
async fn lists_and_calls_host_mcp_tools_with_contract_headers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/mcp/sveda"))
        .respond_with(McpResponder)
        .mount(&server)
        .await;

    let client = HostMcpClient::from_timeout(
        5,
        McpCredentials {
            url: format!("{}/mcp/sveda", server.uri()),
            token: "mcp-secret".into(),
        },
        McpCallContext {
            page_context: Some(json!({ "route": "calendar" })),
            chat_id: Some("chat-1".into()),
        },
    );

    let tools = client.list_tools().await.expect("list");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "get_calendar_events");
    assert_eq!(tools[0].domain, "calendar");
    assert_eq!(
        client.instructions().as_deref(),
        Some("Host tools for the current user.")
    );

    let result = client
        .call_tool("get_calendar_events", json!({ "limit": 1 }))
        .await
        .expect("call");
    assert_eq!(result["success"], true);

    let requests = server.received_requests().await.unwrap();
    assert!(requests.iter().any(|request| {
        request
            .headers
            .get("authorization")
            .map(|value| value.to_str().unwrap())
            == Some("Bearer mcp-secret")
            && request.headers.get("x-sveda-page-context").is_some()
            && request
                .headers
                .get("x-sveda-chat-id")
                .map(|value| value.to_str().unwrap())
                == Some("chat-1")
    }));
}

#[tokio::test]
async fn drops_oversized_page_context_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("authorization", "Bearer t"))
        .respond_with(McpResponder)
        .mount(&server)
        .await;

    let huge = "x".repeat(25000);
    let client = HostMcpClient::from_timeout(
        5,
        McpCredentials {
            url: server.uri(),
            token: "t".into(),
        },
        McpCallContext {
            page_context: Some(json!({ "blob": huge })),
            chat_id: None,
        },
    );
    let _ = client.list_tools().await;
    let requests = server.received_requests().await.unwrap();
    assert!(requests
        .iter()
        .all(|request| request.headers.get("x-sveda-page-context").is_none()));
}
