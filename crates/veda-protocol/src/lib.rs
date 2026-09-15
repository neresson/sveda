use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: &str = "1.0";
pub const SSE_DONE_LINE: &str = "data: [DONE]";
pub const ACCEPT_VEDA_STREAM: &str = "application/vnd.veda.stream+json";
pub const ACCEPT_SSE: &str = "text/event-stream";
pub const HEADER_EMBED_TOKEN: &str = "x-veda-embed-token";
pub const HEADER_HOST_KEY: &str = "x-veda-host-key";
pub const HEADER_PROTOCOL: &str = "x-veda-protocol";
pub const HEADER_PROTOCOL_VERSION: &str = "x-veda-protocol-version";
pub const HEADER_ACCEL_BUFFERING: &str = "x-accel-buffering";
pub const HEADER_PAGE_CONTEXT: &str = "x-veda-page-context";
pub const HEADER_CHAT_ID: &str = "x-veda-chat-id";
pub const HEADER_ADMIN_KEY: &str = "x-veda-admin-key";
pub const TOKEN_PREFIX: &str = "veda_embed_";

pub const STREAM_EVENTS: &[&str] = &[
    "message.start",
    "text.delta",
    "reasoning.delta",
    "tool.call",
    "tool.result",
    "tool.progress",
    "context.usage",
    "chat.title",
    "max_steps",
    "message.end",
    "error",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolTarget {
    Backend,
    Frontend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolProgressTask {
    pub id: String,
    pub label: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message.start")]
    MessageStart {
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "text.delta")]
    TextDelta {
        delta: String,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "reasoning.delta")]
    ReasoningDelta {
        delta: String,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "tool.call")]
    ToolCall {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        target: ToolTarget,
        input: serde_json::Value,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "tool.result")]
    ToolResult {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        #[serde(rename = "toolName")]
        tool_name: String,
        output: serde_json::Value,
        #[serde(rename = "renderHint", skip_serializing_if = "Option::is_none")]
        render_hint: Option<String>,
        #[serde(rename = "renderData", skip_serializing_if = "Option::is_none")]
        render_data: Option<serde_json::Value>,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "tool.progress")]
    ToolProgress {
        #[serde(skip_serializing_if = "Option::is_none")]
        phase: Option<String>,
        tasks: Vec<ToolProgressTask>,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "context.usage")]
    ContextUsage {
        #[serde(rename = "usedTokens")]
        used_tokens: u64,
        #[serde(rename = "maxTokens")]
        max_tokens: u64,
        percent: f64,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "chat.title")]
    ChatTitle {
        title: String,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "max_steps")]
    MaxSteps {
        #[serde(rename = "maxSteps")]
        max_steps: u32,
        #[serde(rename = "canContinue")]
        can_continue: bool,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "message.end")]
    MessageEnd {
        #[serde(rename = "finishReason", skip_serializing_if = "Option::is_none")]
        finish_reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<StreamUsage>,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
        #[serde(rename = "chatId", skip_serializing_if = "Option::is_none")]
        chat_id: Option<String>,
        #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
        message_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<DateTime<Utc>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamUsage {
    #[serde(rename = "promptTokens", skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(rename = "completionTokens", skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    #[serde(rename = "totalTokens", skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

impl StreamEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::MessageStart { .. } => "message.start",
            Self::TextDelta { .. } => "text.delta",
            Self::ReasoningDelta { .. } => "reasoning.delta",
            Self::ToolCall { .. } => "tool.call",
            Self::ToolResult { .. } => "tool.result",
            Self::ToolProgress { .. } => "tool.progress",
            Self::ContextUsage { .. } => "context.usage",
            Self::ChatTitle { .. } => "chat.title",
            Self::MaxSteps { .. } => "max_steps",
            Self::MessageEnd { .. } => "message.end",
            Self::Error { .. } => "error",
        }
    }
}

pub fn encode_sse_event(event: &StreamEvent) -> String {
    format!(
        "data: {}\n\n",
        serde_json::to_string(event).expect("stream event is serializable")
    )
}

pub fn encode_sse_done() -> String {
    format!("{SSE_DONE_LINE}\n\n")
}

pub fn parse_sse_line(line: &str) -> Option<StreamEvent> {
    let trimmed = line.trim();
    let payload = trimmed.strip_prefix("data:")?.trim();
    if payload.is_empty() || payload == "[DONE]" {
        return None;
    }
    serde_json::from_str(payload).ok()
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamRequest {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub messages: Option<Vec<serde_json::Value>>,
    #[serde(rename = "chatId")]
    pub chat_id: Option<String>,
    #[serde(default)]
    pub context: Option<serde_json::Value>,
    #[serde(rename = "clientTools")]
    pub client_tools: Option<Vec<serde_json::Value>>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedTokenRequest {
    pub visitor_id: Option<String>,
    pub host_mcp_url: Option<String>,
    pub host_mcp_token: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbedTokenResponse {
    pub token: String,
    pub visitor_id: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySummary {
    pub id: String,
    #[serde(rename = "chatId")]
    pub chat_id: String,
    pub title: String,
    pub preview: String,
    #[serde(rename = "tokensUsed")]
    pub tokens_used: u64,
    pub version: u32,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryDetail {
    pub id: String,
    #[serde(rename = "chatId")]
    pub chat_id: String,
    pub title: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(rename = "conversationHistory")]
    pub conversation_history: Vec<serde_json::Value>,
    #[serde(rename = "tokensUsed")]
    pub tokens_used: u64,
    pub version: u32,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageResponse {
    pub explanation: String,
    pub tokens_used: u64,
    pub chat_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractItem {
    pub filename: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SidecarContract {
    pub version: String,
    pub prefix: String,
    pub accept: ContractAccept,
    pub headers: ContractHeaders,
    pub routes: Vec<ContractRoute>,
    #[serde(rename = "streamEvents")]
    pub stream_events: Vec<String>,
    #[serde(rename = "sseDoneLine")]
    pub sse_done_line: String,
    #[serde(rename = "embedToken")]
    pub embed_token: ContractEmbedToken,
    pub message: ContractMessage,
    pub histories: ContractHistories,
    #[serde(rename = "documentsExtract")]
    pub documents_extract: ContractDocumentsExtract,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractAccept {
    #[serde(rename = "vedaStream")]
    pub veda_stream: String,
    pub sse: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractHeaders {
    pub inbound: Vec<String>,
    #[serde(rename = "streamResponse")]
    pub stream_response: std::collections::HashMap<String, String>,
    #[serde(rename = "mcpOutbound")]
    pub mcp_outbound: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractRoute {
    pub method: String,
    pub path: String,
    pub mode: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractEmbedToken {
    pub request: Vec<String>,
    #[serde(rename = "responseRequired")]
    pub response_required: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractMessage {
    #[serde(rename = "responseRequired")]
    pub response_required: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractHistories {
    #[serde(rename = "listKey")]
    pub list_key: String,
    #[serde(rename = "detailKey")]
    pub detail_key: String,
    #[serde(rename = "summaryRequired")]
    pub summary_required: Vec<String>,
    #[serde(rename = "detailRequired")]
    pub detail_required: Vec<String>,
    #[serde(rename = "patchRequest")]
    pub patch_request: Vec<String>,
    #[serde(rename = "mutationResponseRequired")]
    pub mutation_response_required: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContractDocumentsExtract {
    #[serde(rename = "requestField")]
    pub request_field: String,
    #[serde(rename = "responseKey")]
    pub response_key: String,
    #[serde(rename = "itemRequired")]
    pub item_required: Vec<String>,
}

pub fn load_sidecar_contract(path: impl AsRef<std::path::Path>) -> SidecarContract {
    let raw = std::fs::read_to_string(path).expect("sidecar contract must exist");
    serde_json::from_str(&raw).expect("sidecar contract must be valid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> SidecarContract {
        load_sidecar_contract(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../packages/protocol/contracts/sidecar.v1.json"
        ))
    }

    #[test]
    fn protocol_constants_match_sidecar_contract() {
        let contract = contract();
        assert_eq!(contract.version, PROTOCOL_VERSION);
        assert_eq!(contract.accept.veda_stream, ACCEPT_VEDA_STREAM);
        assert_eq!(contract.accept.sse, ACCEPT_SSE);
        assert_eq!(contract.sse_done_line, SSE_DONE_LINE);
        assert_eq!(
            contract
                .headers
                .stream_response
                .get("X-Veda-Protocol-Version"),
            Some(&PROTOCOL_VERSION.to_string())
        );
        assert_eq!(
            contract.stream_events,
            STREAM_EVENTS
                .iter()
                .map(|event| event.to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn sse_round_trips_hello_world_events() {
        let chat_id = Some("chat-1".to_string());
        let events = [
            StreamEvent::MessageStart {
                chat_id: chat_id.clone(),
                message_id: Some("m1".into()),
                timestamp: None,
            },
            StreamEvent::TextDelta {
                delta: "Hello".into(),
                chat_id: chat_id.clone(),
                message_id: Some("m1".into()),
                timestamp: None,
            },
            StreamEvent::MessageEnd {
                finish_reason: Some("stop".into()),
                usage: Some(StreamUsage {
                    prompt_tokens: Some(1),
                    completion_tokens: Some(1),
                    total_tokens: Some(2),
                }),
                chat_id,
                message_id: Some("m1".into()),
                timestamp: None,
            },
        ];

        for event in events {
            let encoded = encode_sse_event(&event);
            assert!(encoded.starts_with("data: "));
            assert!(encoded.ends_with("\n\n"));
            let parsed = parse_sse_line(&encoded).expect("event should parse");
            assert_eq!(parsed.event_type(), event.event_type());
            assert!(STREAM_EVENTS.contains(&parsed.event_type()));
        }

        assert_eq!(encode_sse_done(), "data: [DONE]\n\n");
        assert!(parse_sse_line("data: [DONE]").is_none());
    }

    #[test]
    fn wire_events_round_trip_c2_payloads() {
        let events = [
            StreamEvent::ReasoningDelta {
                delta: "think".into(),
                chat_id: None,
                message_id: None,
                timestamp: None,
            },
            StreamEvent::ContextUsage {
                used_tokens: 150,
                max_tokens: 1000,
                percent: 15.0,
                chat_id: None,
                message_id: None,
                timestamp: None,
            },
            StreamEvent::ChatTitle {
                title: "Ocean tides".into(),
                chat_id: None,
                message_id: None,
                timestamp: None,
            },
            StreamEvent::MaxSteps {
                max_steps: 1,
                can_continue: true,
                chat_id: None,
                message_id: None,
                timestamp: None,
            },
            StreamEvent::ToolCall {
                tool_call_id: "call-1".into(),
                tool_name: "confirm_course_creation".into(),
                target: ToolTarget::Frontend,
                input: serde_json::json!({"title": "Math"}),
                chat_id: None,
                message_id: None,
                timestamp: None,
            },
            StreamEvent::ToolResult {
                tool_call_id: "call-1".into(),
                tool_name: "get_calendar_events".into(),
                output: serde_json::json!({"success": true}),
                render_hint: None,
                render_data: None,
                chat_id: None,
                message_id: None,
                timestamp: None,
            },
            StreamEvent::ToolProgress {
                phase: None,
                tasks: vec![ToolProgressTask {
                    id: "task-1".into(),
                    label: "Search".into(),
                    status: "running".into(),
                    detail: None,
                }],
                chat_id: None,
                message_id: None,
                timestamp: None,
            },
        ];

        for event in events {
            let parsed = parse_sse_line(&encode_sse_event(&event)).expect("parse");
            assert_eq!(parsed.event_type(), event.event_type());
            assert!(STREAM_EVENTS.contains(&parsed.event_type()));
        }
    }
}
