use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use sha2::{Digest, Sha256};
use tokio::sync::OnceCell;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Client;
use serde_json::{json, Value};
use sveda_protocol::{HEADER_CHAT_ID, HEADER_PAGE_CONTEXT};

pub const PAGE_CONTEXT_MAX_BYTES: usize = 24000;
pub const MCP_PROTOCOL_VERSION: &str = "2026-07-28";
const LEGACY_VERSION: &str = "2025-11-25";

#[derive(Debug, Clone, thiserror::Error)]
pub enum McpError {
    #[error("server requires legacy MCP initialization")]
    LegacyDiscovery,
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
    pub always_loaded: bool,
    pub approval: McpApproval,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum McpApproval {
    #[default]
    Unspecified,
    Always,
    Batch,
    Guarded,
    Course,
    Auto,
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
    version: OnceCell<&'static str>,
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
            version: OnceCell::new(),
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
        self.version.get_or_try_init(|| self.discover()).await?;
        Ok(())
    }

    async fn discover(&self) -> Result<&'static str, McpError> {
        match self
            .send_rpc("server/discover", json!({}), MCP_PROTOCOL_VERSION, None)
            .await
        {
            Ok(response) => {
                let versions = response
                    .body
                    .pointer("/result/supportedVersions")
                    .and_then(Value::as_array)
                    .ok_or_else(|| McpError::Other("invalid MCP discovery response".into()))?;
                if versions.iter().any(|v| v == MCP_PROTOCOL_VERSION) {
                    self.store_instructions(&response.body);
                    return Ok(MCP_PROTOCOL_VERSION);
                }
                if !versions.iter().any(|v| v == LEGACY_VERSION) {
                    return Err(McpError::Other("incompatible MCP server version".into()));
                }
            }
            Err(McpError::LegacyDiscovery) => {}
            Err(error) => return Err(error),
        }
        let response = self
            .send_rpc(
                "initialize",
                json!({
                    "protocolVersion": LEGACY_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "sveda", "version": "0.1.0" }
                }),
                LEGACY_VERSION,
                None,
            )
            .await?;
        if response
            .body
            .pointer("/result/protocolVersion")
            .and_then(Value::as_str)
            != Some(LEGACY_VERSION)
        {
            return Err(McpError::Other(
                "incompatible legacy MCP server version".into(),
            ));
        }
        if let Some(session) = response.session_id {
            *self.session_id.lock().expect("session") = Some(session);
        }
        self.store_instructions(&response.body);
        self.post(
            json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            LEGACY_VERSION,
            None,
        )
        .await?;
        Ok(LEGACY_VERSION)
    }

    fn store_instructions(&self, body: &Value) {
        if let Some(text) = body
            .pointer("/result/instructions")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            *self.instructions.lock().expect("instructions") = Some(text.to_string());
        }
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
            let payload = self
                .send_rpc("tools/list", params, self.version.get().unwrap(), None)
                .await?;
            let page = payload
                .body
                .pointer("/result/tools")
                .and_then(Value::as_array)
                .ok_or_else(|| McpError::Other("invalid MCP tools/list response".into()))?;
            tools.extend(page.iter().filter_map(parse_tool));
            cursor = payload
                .body
                .pointer("/result/nextCursor")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            if cursor.is_none() {
                return Ok(tools);
            }
        }
        Err(McpError::Other(
            "MCP tool catalog exceeded the pagination limit".into(),
        ))
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, McpError> {
        self.call_tool_with_id(name, arguments, None).await
    }

    pub async fn call_tool_with_id(
        &self,
        name: &str,
        arguments: Value,
        call_id: Option<&str>,
    ) -> Result<Value, McpError> {
        let key = call_id.map(|id| self.idempotency_key("sveda", id));
        let payload = self
            .rpc(
                "tools/call",
                json!({
                    "name": name,
                    "arguments": arguments,
                }),
                key.as_deref(),
            )
            .await?;
        tool_result(&payload.body)
    }

    pub async fn approval_scope(&self, name: &str, arguments: &Value) -> Option<String> {
        let response = self
            .rpc(
                "sveda/approval/check",
                json!({ "name": name, "arguments": arguments }),
                None,
            )
            .await
            .ok()?;
        response
            .body
            .pointer("/result/scope")?
            .as_str()
            .filter(|scope| !scope.is_empty() && scope.len() <= 512)
            .map(ToOwned::to_owned)
    }

    // A rejecting host guarantees that no tool was executed. For ambiguous transport
    // failures, propagate the error instead of asking for a potentially duplicate write.
    pub async fn call_with_approval(
        &self,
        name: &str,
        arguments: &Value,
        scope: &str,
        call_id: &str,
        legacy: bool,
    ) -> Result<Option<Value>, McpError> {
        // Keep legacy keys stable. New policies share a key with tools/call
        // so changing eligibility cannot make a retry repeat a write.
        let key = self.idempotency_key(if legacy { "sveda-approval" } else { "sveda" }, call_id);
        let payload = self
            .rpc(
                "sveda/approval/call",
                json!({ "name": name, "arguments": arguments, "scope": scope }),
                Some(&key),
            )
            .await?;
        if payload.body.pointer("/result/allowed") == Some(&Value::Bool(false))
            || payload.body.pointer("/error/code").and_then(Value::as_i64) == Some(-32601)
        {
            return Ok(None);
        }
        tool_result(&payload.body).map(Some)
    }

    pub fn approval_host(&self) -> &str {
        &self.url
    }

    fn idempotency_key(&self, prefix: &str, id: &str) -> String {
        let chat = self.context.chat_id.as_deref().unwrap_or_default();
        let digest = Sha256::digest(format!("{}:{chat}{id}", chat.len()).as_bytes());
        format!("{prefix}-{digest:x}")
    }

    async fn rpc(
        &self,
        method: &str,
        params: Value,
        key: Option<&str>,
    ) -> Result<RpcResponse, McpError> {
        let version = self.version.get_or_try_init(|| self.discover()).await?;
        self.send_rpc(method, params, version, key).await
    }

    async fn send_rpc(
        &self,
        method: &str,
        mut params: Value,
        version: &str,
        key: Option<&str>,
    ) -> Result<RpcResponse, McpError> {
        if version == MCP_PROTOCOL_VERSION {
            params["_meta"] = json!({
                "io.modelcontextprotocol/protocolVersion": MCP_PROTOCOL_VERSION,
                "io.modelcontextprotocol/clientCapabilities": {},
                "io.modelcontextprotocol/clientInfo": {"name":"sveda","version":env!("CARGO_PKG_VERSION")}
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.post(body, version, key).await
    }

    async fn post(
        &self,
        body: Value,
        version: &str,
        key: Option<&str>,
    ) -> Result<RpcResponse, McpError> {
        let method = body["method"].as_str().unwrap_or_default();
        let mut request = self
            .http
            .post(&self.url)
            .headers(self.headers()?)
            .header("MCP-Protocol-Version", version)
            .json(&body);
        if version == MCP_PROTOCOL_VERSION {
            request = request.header("Mcp-Method", method);
            if let Some(name) = body.pointer("/params/name").and_then(Value::as_str) {
                request = request.header("Mcp-Name", name);
            }
        }
        if let Some(key) = key {
            request = request.header("Idempotency-Key", key);
        }
        if let Some(session) = self.session_id.lock().expect("session").clone() {
            request = request.header("mcp-session-id", session);
        }
        let response = request
            .send()
            .await
            .map_err(|error| McpError::Other(error.without_url().to_string()))?;
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
            .map_err(|error| McpError::Other(error.without_url().to_string()))?;
        if method == "server/discover" && (status.is_success() || status.as_u16() == 400) {
            let parsed = parse_rpc_body(&bytes, &content_type).ok();
            let code = parsed
                .as_ref()
                .and_then(|v| v.pointer("/error/code"))
                .and_then(Value::as_i64);
            let unsupported = code == Some(-32022)
                && parsed
                    .as_ref()
                    .and_then(|v| v.pointer("/error/data/supported"))
                    .and_then(Value::as_array)
                    .is_some_and(|versions| versions.iter().any(|v| v == LEGACY_VERSION));
            if matches!(code, Some(-32601 | -32600))
                || unsupported
                || (status.as_u16() == 400 && !matches!(code, Some(-32022 | -32020 | -32021)))
            {
                return Err(McpError::LegacyDiscovery);
            }
        }
        if !status.is_success() {
            return Err(McpError::Other(format!(
                "MCP {method} returned HTTP {}",
                status.as_u16()
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
    if content_type.contains("text/event-stream") || text.trim_start().starts_with("data:") {
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
    let annotations = value.get("annotations").unwrap_or(&Value::Null);
    let is_read = annotations.get("readOnlyHint").and_then(Value::as_bool) == Some(true);
    let destructive = mode == "delete"
        || annotations.get("destructiveHint").and_then(Value::as_bool) == Some(true);
    let open_world =
        !is_read && annotations.get("openWorldHint").and_then(Value::as_bool) == Some(true);
    let approval = match (
        open_world || destructive,
        meta.get("confirm").and_then(Value::as_str),
    ) {
        _ if open_world => McpApproval::Always,
        // A resource-scoped write still requires confirmation and host revalidation,
        // including eligible destructive operations such as deleting a draft.
        (_, Some("once_per_course_per_chat")) => McpApproval::Course,
        (true, _) | (_, Some("always")) => McpApproval::Always,
        (_, Some("batch")) => McpApproval::Batch,
        (_, Some("once_per_chat")) => McpApproval::Guarded,
        (_, Some("auto_if_host_allows")) => McpApproval::Auto,
        _ => McpApproval::Unspecified,
    };
    let confirmation_required = approval != McpApproval::Unspecified
        || meta.get("confirmation").and_then(Value::as_str) == Some("required");
    Some(McpTool {
        name,
        description,
        input_schema,
        domain,
        mode,
        confirmation_required,
        always_loaded: meta.get("ai.sveda/alwaysLoaded").and_then(Value::as_bool) == Some(true),
        approval,
    })
}

fn tool_result(body: &Value) -> Result<Value, McpError> {
    if !body["result"].is_object() && !body["error"].is_object() {
        return Err(McpError::Other("missing MCP tool result".into()));
    }
    Ok(map_call_result(body))
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
