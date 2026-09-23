use serde_json::Value;

use crate::{LlmChunk, Protocol};

struct Frame {
    event: String,
    data: String,
}

pub struct ProviderParser {
    protocol: Protocol,
    prompt_tokens: u64,
    completion_tokens: u64,
    finish_reason: String,
    saw_end: bool,
    flushed: bool,
    got_reasoning: bool,
}

impl ProviderParser {
    pub fn new(protocol: Protocol) -> Self {
        Self {
            protocol,
            prompt_tokens: 0,
            completion_tokens: 0,
            finish_reason: "stop".into(),
            saw_end: false,
            flushed: false,
            got_reasoning: false,
        }
    }

    pub fn push(&mut self, event: &str, data: &str) -> Vec<LlmChunk> {
        let mut chunks = match self.protocol {
            Protocol::Responses => parse_responses(
                data,
                &mut self.prompt_tokens,
                &mut self.completion_tokens,
                &mut self.finish_reason,
                &mut self.saw_end,
                &mut self.got_reasoning,
            ),
            Protocol::Anthropic => parse_anthropic(
                event,
                data,
                &mut self.prompt_tokens,
                &mut self.completion_tokens,
                &mut self.finish_reason,
                &mut self.saw_end,
            )
            .into_iter()
            .collect(),
        };
        chunks.extend(self.flush_end());
        chunks
    }

    fn flush_end(&mut self) -> Vec<LlmChunk> {
        if !self.saw_end || self.flushed {
            return Vec::new();
        }
        self.flushed = true;
        vec![
            LlmChunk::Usage {
                prompt_tokens: self.prompt_tokens,
                completion_tokens: self.completion_tokens,
            },
            LlmChunk::End {
                finish_reason: self.finish_reason.clone(),
            },
        ]
    }
}

pub fn parse_provider_sse(protocol: Protocol, body: &str) -> Vec<LlmChunk> {
    let mut parser = ProviderParser::new(protocol);
    let mut chunks = Vec::new();
    for frame in frames(body) {
        chunks.extend(parser.push(&frame.event, &frame.data));
    }
    chunks
}

fn frames(body: &str) -> Vec<Frame> {
    let mut frames = Vec::new();
    let mut event = String::new();
    let mut data_lines = Vec::new();
    for line in body.lines() {
        if line.is_empty() {
            if !data_lines.is_empty() {
                frames.push(Frame {
                    event: std::mem::take(&mut event),
                    data: data_lines.join("\n"),
                });
                data_lines.clear();
            }
            event.clear();
            continue;
        }
        if let Some(value) = line.strip_prefix("event:") {
            event = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("data:") {
            data_lines.push(value.trim().to_string());
        }
    }
    if !data_lines.is_empty() {
        frames.push(Frame {
            event,
            data: data_lines.join("\n"),
        });
    }
    frames
}

fn parse_responses(
    data: &str,
    prompt_tokens: &mut u64,
    completion_tokens: &mut u64,
    finish_reason: &mut String,
    saw_end: &mut bool,
    got_reasoning: &mut bool,
) -> Vec<LlmChunk> {
    let Ok(value) = serde_json::from_str::<Value>(data) else {
        return Vec::new();
    };
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "response.created" => {
            *got_reasoning = false;
            Vec::new()
        }
        "response.output_text.delta" => delta_text(&value)
            .map(LlmChunk::TextDelta)
            .into_iter()
            .collect(),
        "response.reasoning_text.delta" | "response.reasoning.delta" => {
            if let Some(text) = delta_text(&value) {
                *got_reasoning = true;
                vec![LlmChunk::ReasoningDelta(text)]
            } else {
                Vec::new()
            }
        }
        "response.reasoning_text.done" | "response.reasoning.done" => {
            if *got_reasoning {
                return Vec::new();
            }
            if let Some(text) = reasoning_done_text(&value) {
                *got_reasoning = true;
                vec![LlmChunk::ReasoningDelta(text)]
            } else {
                Vec::new()
            }
        }
        "response.output_item.done" => {
            let Some(item) = value.get("item") else {
                return Vec::new();
            };
            if let Some(chunk) = parse_function_call(item) {
                *finish_reason = "tool_calls".into();
                return vec![chunk];
            }
            if *got_reasoning {
                return Vec::new();
            }
            if let Some(text) = reasoning_item_text(item) {
                *got_reasoning = true;
                vec![LlmChunk::ReasoningDelta(text)]
            } else {
                Vec::new()
            }
        }
        "response.completed" => {
            let mut extra = Vec::new();
            if let Some(response) = value.get("response") {
                if let Some(usage) = response.get("usage") {
                    *prompt_tokens = token(usage, &["input_tokens", "prompt_tokens"]);
                    *completion_tokens = token(usage, &["output_tokens", "completion_tokens"]);
                }
                if let Some(output) = response.get("output").and_then(Value::as_array) {
                    if !*got_reasoning {
                        if let Some(text) = output.iter().find_map(reasoning_item_text) {
                            *got_reasoning = true;
                            extra.push(LlmChunk::ReasoningDelta(text));
                        }
                    }
                    if finish_reason.as_str() != "tool_calls" {
                        extra.extend(output.iter().filter_map(parse_function_call));
                        *finish_reason = if extra
                            .iter()
                            .any(|chunk| matches!(chunk, LlmChunk::ToolCall { .. }))
                        {
                            "tool_calls".into()
                        } else if response.get("status").and_then(Value::as_str)
                            == Some("incomplete")
                        {
                            "length".into()
                        } else {
                            "stop".into()
                        };
                    }
                } else if finish_reason.as_str() != "tool_calls" {
                    *finish_reason =
                        if response.get("status").and_then(Value::as_str) == Some("incomplete") {
                            "length".into()
                        } else {
                            "stop".into()
                        };
                }
            }
            *saw_end = true;
            extra
        }
        "response.failed" | "error" => {
            *finish_reason = "error".into();
            *saw_end = true;
            Vec::new()
        }
        _ => Vec::new(),
    }
}

fn parse_anthropic(
    event: &str,
    data: &str,
    prompt_tokens: &mut u64,
    completion_tokens: &mut u64,
    finish_reason: &mut String,
    saw_end: &mut bool,
) -> Option<LlmChunk> {
    let value: Value = serde_json::from_str(data).ok()?;
    let kind = if event.is_empty() {
        value.get("type").and_then(Value::as_str).unwrap_or("")
    } else {
        event
    };
    match kind {
        "message_start" => {
            if let Some(usage) = value
                .get("message")
                .and_then(|message| message.get("usage"))
            {
                *prompt_tokens = token(usage, &["input_tokens"]);
            }
            None
        }
        "content_block_delta" => {
            let delta = value.get("delta")?;
            match delta.get("type").and_then(Value::as_str) {
                Some("thinking_delta") => delta
                    .get("thinking")
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                    .map(|text| LlmChunk::ReasoningDelta(text.to_string())),
                Some("text_delta") => delta
                    .get("text")
                    .and_then(Value::as_str)
                    .filter(|text| !text.is_empty())
                    .map(|text| LlmChunk::TextDelta(text.to_string())),
                _ => None,
            }
        }
        "content_block_start" => {
            let block = value.get("content_block")?;
            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                parse_function_call(block)
            } else {
                None
            }
        }
        "message_delta" => {
            if let Some(usage) = value.get("usage") {
                *completion_tokens = token(usage, &["output_tokens"]);
            }
            if let Some(reason) = value.pointer("/delta/stop_reason").and_then(Value::as_str) {
                *finish_reason = map_anthropic_stop(reason).to_string();
            }
            None
        }
        "message_stop" => {
            *saw_end = true;
            None
        }
        "error" => {
            *finish_reason = "error".into();
            *saw_end = true;
            None
        }
        _ => None,
    }
}

fn reasoning_item_text(item: &Value) -> Option<String> {
    if item.get("type").and_then(Value::as_str) != Some("reasoning") {
        return None;
    }
    if let Some(text) = item.get("content").and_then(Value::as_str) {
        let trimmed = text.trim();
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }
    let parts = item.get("content").and_then(Value::as_array)?;
    let mut out = String::new();
    for part in parts {
        let text = part
            .get("text")
            .and_then(Value::as_str)
            .or_else(|| part.as_str())
            .unwrap_or("");
        out.push_str(text);
    }
    let trimmed = out.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn reasoning_done_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("text").and_then(Value::as_str) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    value.get("item").and_then(reasoning_item_text)
}

fn parse_function_call(item: &Value) -> Option<LlmChunk> {
    let kind = item.get("type").and_then(Value::as_str)?;
    if kind != "function_call" && kind != "tool_use" {
        return None;
    }
    let id = item
        .get("call_id")
        .or_else(|| item.get("id"))
        .and_then(Value::as_str)?
        .to_string();
    let name = item.get("name").and_then(Value::as_str)?.to_string();
    let input = item.get("input").cloned().unwrap_or_else(|| {
        item.get("arguments")
            .and_then(Value::as_str)
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| Value::Object(Default::default()))
    });
    Some(LlmChunk::ToolCall { id, name, input })
}

fn delta_text(value: &Value) -> Option<String> {
    value
        .get("delta")
        .and_then(|delta| {
            delta.as_str().map(ToOwned::to_owned).or_else(|| {
                delta
                    .get("text")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
            })
        })
        .or_else(|| {
            value
                .get("text")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .filter(|text| !text.is_empty())
}

fn token(usage: &Value, keys: &[&str]) -> u64 {
    keys.iter()
        .find_map(|key| usage.get(*key).and_then(Value::as_u64))
        .unwrap_or(0)
}

fn map_anthropic_stop(reason: &str) -> &'static str {
    match reason {
        "tool_use" => "tool_calls",
        "max_tokens" => "length",
        "pause_turn" => "continue",
        "end_turn" | "stop_sequence" => "stop",
        _ => "stop",
    }
}
