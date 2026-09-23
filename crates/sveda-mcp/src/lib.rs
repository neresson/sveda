use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde_json::{json, Value};
use sveda_protocol::{HEADER_CHAT_ID, HEADER_PAGE_CONTEXT};

pub const PAGE_CONTEXT_MAX_BYTES: usize = 24000;
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

#[derive(Debug, Clone, thiserror::Error)]
pub enum McpError {
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone)]
pub struct McpCredentials {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub domain: String,
    pub mode: String,
    pub confirmation_required: bool,
}

#[derive(Debug, Clone)]
pub struct McpCallContext {
    pub page_context: Option<Value>,
    pub chat_id: Option<String>,
}

pub struct HostMcpClient {
    http: Client,
    url: String,
    token: String,
    session_id: Mutex<Option<String>>,
    next_id: AtomicU64,
    context: McpCallContext,
    instructions: Mutex<Option<String>>,
}

impl HostMcpClient {
    pub fn new(http: Client, creds: McpCredentials, context: McpCallContext) -> Self {
        Self {
            http,
            url: creds.url.trim_end_matches('/').to_string(),
            token: creds.token,
            session_id: Mutex::new(None),
            next_id: AtomicU64::new(1),
            context,
            instructions: Mutex::new(None),
        }
    }

    pub fn from_timeout(seconds: u64, creds: McpCredentials, context: McpCallContext) -> Self {
        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(seconds.max(1)))
            .build()
            .expect("reqwest client");
        Self::new(http, creds, context)
    }

    pub async fn initialize(&self) -> Result<(), McpError> {
        let response = self
            .rpc(
                "initialize",
                json!({
                    "protocolVersion": MCP_PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "sveda", "version": "0.1.0" }
                }),
            )
            .await?;
        if let Some(session) = response.session_id {
            *self.session_id.lock().expect("session") = Some(session);
        }
        if let Some(text) = response
            .body
            .pointer("/result/instructions")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            *self.instructions.lock().expect("instructions") = Some(text.to_string());
        }
        let _ = self
            .rpc_notification("notifications/initialized", json!({}))
            .await;
        Ok(())
    }

    pub fn instructions(&self) -> Option<String> {
        self.instructions
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        self.initialize().await?;
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..20 {
            let mut params = json!({ "per_page": 250 });
            if let Some(value) = &cursor {
                params["cursor"] = json!(value);
            }
            let payload = self.rpc("tools/list", params).await?;
            let page = payload
                .body
                .pointer("/result/tools")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            tools.extend(page.iter().filter_map(parse_tool));
            cursor = payload
                .body
                .pointer("/result/nextCursor")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            if cursor.is_none() {
                break;
            }
        }
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, McpError> {
        if self.session_id.lock().expect("session").is_none() {
            self.initialize().await?;
        }
        let payload = self
            .rpc(
                "tools/call",
                json!({
                    "name": name,
                    "arguments": arguments,
                }),
            )
            .await?;
        Ok(map_call_result(&payload.body))
    }

    async fn rpc(&self, method: &str, params: Value) -> Result<RpcResponse, McpError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.post(body).await
    }

    async fn rpc_notification(&self, method: &str, params: Value) -> Result<(), McpError> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let _ = self.post(body).await;
        Ok(())
    }

    async fn post(&self, body: Value) -> Result<RpcResponse, McpError> {
        let mut request = self
            .http
            .post(&self.url)
            .headers(self.headers()?)
            .json(&body);
        if let Some(session) = self.session_id.lock().expect("session").clone() {
            request = request.header("mcp-session-id", session);
        }
        let response = request
            .send()
            .await
            .map_err(|error| McpError::Other(error.to_string()))?;
        let session_id = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let status = response.status();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| McpError::Other(error.to_string()))?;
        if !status.is_success() && status.as_u16() != 202 {
            return Err(McpError::Other(format!(
                "mcp http {}: {}",
                status.as_u16(),
                String::from_utf8_lossy(&bytes)
            )));
        }
        if bytes.is_empty() {
            return Ok(RpcResponse {
                body: json!({}),
                session_id,
            });
        }
        let parsed = parse_rpc_body(&bytes, &content_type)?;
        Ok(RpcResponse {
            body: parsed,
            session_id,
        })
    }

    fn headers(&self) -> Result<HeaderMap, McpError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "mcp-protocol-version",
            HeaderValue::from_static(MCP_PROTOCOL_VERSION),
        );
        let token = self.token.trim();
        if !token.is_empty() {
            let auth = format!("Bearer {token}");
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth).map_err(|error| McpError::Other(error.to_string()))?,
            );
        }
        if let Some(context) = &self.context.page_context {
            if let Ok(encoded) = serde_json::to_string(context) {
                if encoded.len() <= PAGE_CONTEXT_MAX_BYTES {
                    if let Ok(value) = HeaderValue::from_str(&encoded) {
                        headers.insert(HeaderName::from_static(HEADER_PAGE_CONTEXT), value);
                    }
                }
            }
        }
        if let Some(chat_id) = self
            .context
            .chat_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if let Ok(value) = HeaderValue::from_str(chat_id) {
                headers.insert(HeaderName::from_static(HEADER_CHAT_ID), value);
            }
        }
        Ok(headers)
    }
}

struct RpcResponse {
    body: Value,
    session_id: Option<String>,
}

fn parse_rpc_body(bytes: &[u8], content_type: &str) -> Result<Value, McpError> {
    let text = String::from_utf8_lossy(bytes);
    if content_type.contains("text/event-stream") || text.contains("data:") {
        let mut data = Vec::new();
        for line in text.lines() {
            if let Some(payload) = line.strip_prefix("data:") {
                data.push(payload.trim());
            }
        }
        let joined = data.join("\n");
        return serde_json::from_str(&joined).map_err(|error| McpError::Other(error.to_string()));
    }
    serde_json::from_str(&text).map_err(|error| McpError::Other(error.to_string()))
}

fn parse_tool(value: &Value) -> Option<McpTool> {
    let name = value.get("name")?.as_str()?.to_string();
    let description = value
        .get("description")
        .and_then(Value::as_str)
        .or_else(|| value.get("title").and_then(Value::as_str))
        .unwrap_or(&name)
        .to_string();
    let input_schema = value
        .get("inputSchema")
        .cloned()
        .unwrap_or_else(|| json!({ "type": "object", "properties": {} }));
    let meta = value.get("_meta").cloned().unwrap_or(Value::Null);
    let domain = meta
        .get("domain")
        .and_then(Value::as_str)
        .unwrap_or("other")
        .to_string();
    let mode = meta
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("read")
        .to_string();
    let confirmation_required = meta.get("confirmation").and_then(Value::as_str) == Some("required");
    Some(McpTool {
        name,
        description,
        input_schema,
        domain,
        mode,
        confirmation_required,
    })
}

fn map_call_result(body: &Value) -> Value {
    if let Some(error) = body.get("error") {
        return json!({
            "success": false,
            "error": error.get("message").and_then(Value::as_str).unwrap_or("MCP error."),
        });
    }
    let result = body.get("result").cloned().unwrap_or(Value::Null);
    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        let text = first_text(&result);
        return json!({
            "success": false,
            "error": if text.is_empty() { "MCP tool error.".into() } else { text },
        });
    }
    if let Some(structured) = result.get("structuredContent") {
        if !structured.is_null() && structured != &json!([]) {
            return structured.clone();
        }
    }
    let text = first_text(&result);
    if text.is_empty() {
        return json!({ "success": true, "data": null });
    }
    serde_json::from_str(&text).unwrap_or(Value::String(text))
}

fn first_text(result: &Value) -> String {
    result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find_map(|item| {
                if item.get("type").and_then(Value::as_str) == Some("text") {
                    item.get("text")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
}

pub fn page_context_header_value(context: &Value) -> Option<String> {
    let encoded = serde_json::to_string(context).ok()?;
    if encoded.len() <= PAGE_CONTEXT_MAX_BYTES {
        Some(encoded)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tool_reads_confirmation_meta() {
        let tool = parse_tool(&json!({
            "name": "delete_post",
            "description": "Delete a post",
            "inputSchema": { "type": "object" },
            "_meta": { "domain": "posts", "mode": "delete", "confirmation": "required" }
        }))
        .expect("tool");
        assert!(tool.confirmation_required);
        assert_eq!(tool.mode, "delete");

        let automatic = parse_tool(&json!({
            "name": "search_posts",
            "_meta": { "domain": "posts", "mode": "read" }
        }))
        .expect("tool");
        assert!(!automatic.confirmation_required);
    }
}
