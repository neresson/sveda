use serde_json::{json, Value};
use veda_llm::{ChatMessage, StoredToolCall};

pub fn split_prompt_and_history(
    prompt: Option<&str>,
    messages: &[Value],
) -> (String, Vec<ChatMessage>) {
    let stripped = strip_trailing_empty_assistant(messages);
    if stripped.is_empty() {
        return (prompt.unwrap_or("").trim().to_string(), Vec::new());
    }

    let last_role = stripped
        .last()
        .and_then(|message| message.get("role"))
        .and_then(|role| role.as_str())
        .unwrap_or("");

    if last_role == "user" {
        let prompt = message_text(stripped.last().unwrap());
        let history = stripped[..stripped.len() - 1]
            .iter()
            .filter_map(to_chat_message)
            .collect();
        return (prompt, history);
    }

    if last_role == "tool" {
        return (
            String::new(),
            stripped.iter().filter_map(to_chat_message).collect(),
        );
    }

    (
        prompt.unwrap_or("").trim().to_string(),
        stripped.iter().filter_map(to_chat_message).collect(),
    )
}

pub fn strip_trailing_empty_assistant(messages: &[Value]) -> Vec<Value> {
    let mut items = messages.to_vec();
    while let Some(last) = items.last() {
        if last.get("role").and_then(|role| role.as_str()) != Some("assistant") {
            break;
        }
        if message_text(last).is_empty() && tool_calls_from(last).is_empty() {
            items.pop();
            continue;
        }
        break;
    }
    items
}

pub fn message_text(message: &Value) -> String {
    if let Some(content) = message.get("content").and_then(|value| value.as_str()) {
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let Some(parts) = message.get("parts").and_then(|value| value.as_array()) else {
        return String::new();
    };
    let texts: Vec<&str> = parts
        .iter()
        .filter(|part| part.get("type").and_then(|value| value.as_str()) == Some("text"))
        .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect();
    texts.join("\n")
}

pub fn to_chat_message(message: &Value) -> Option<ChatMessage> {
    let role = message.get("role")?.as_str()?.to_string();
    Some(ChatMessage {
        role,
        content: message_text(message),
        tool_calls: tool_calls_from(message),
        tool_call_id: tool_call_id_from(message),
    })
}

pub fn chat_to_json(message: &ChatMessage) -> Value {
    let mut value = json!({
        "role": message.role,
        "content": message.content,
    });
    if !message.tool_calls.is_empty() {
        value["tool_calls"] = json!(message
            .tool_calls
            .iter()
            .map(|call| json!({
                "id": call.id,
                "function": {
                    "name": call.name,
                    "arguments": call.arguments,
                }
            }))
            .collect::<Vec<_>>());
    }
    if let Some(id) = &message.tool_call_id {
        value["tool_call_id"] = json!(id);
    }
    value
}

pub fn json_to_chat(message: &Value) -> Option<ChatMessage> {
    to_chat_message(message).or_else(|| {
        let role = message.get("role")?.as_str()?.to_string();
        Some(ChatMessage {
            role,
            content: message
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            tool_calls: tool_calls_from(message),
            tool_call_id: message
                .get("tool_call_id")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
        })
    })
}

pub fn display_messages(
    incoming: &[Value],
    prompt: &str,
    assistant: &str,
    assistant_id: &str,
) -> Vec<Value> {
    let mut messages = strip_trailing_empty_assistant(incoming);
    if messages.is_empty() && !prompt.trim().is_empty() {
        messages.push(json!({
            "id": "user_turn",
            "role": "user",
            "content": prompt,
            "parts": [{ "type": "text", "text": prompt }],
        }));
    }
    if !assistant.trim().is_empty() {
        messages.push(json!({
            "id": assistant_id,
            "role": "assistant",
            "content": assistant,
            "parts": [{ "type": "text", "text": assistant }],
        }));
    }
    messages
}

pub fn preview_from(messages: &[Value], prompt: &str) -> String {
    for message in messages {
        if message.get("role").and_then(|role| role.as_str()) != Some("user") {
            continue;
        }
        let text = message_text(message);
        if !text.is_empty() {
            return clip(&text, 160);
        }
    }
    clip(prompt, 160)
}

pub fn conversation_json(history: &[ChatMessage], prompt: &str, assistant: &str) -> Vec<Value> {
    let mut items: Vec<Value> = history.iter().map(chat_to_json).collect();
    if !prompt.trim().is_empty() {
        items.push(json!({
            "role": "user",
            "content": prompt,
        }));
    }
    if !assistant.trim().is_empty() {
        items.push(json!({
            "role": "assistant",
            "content": assistant,
        }));
    }
    items
}

fn clip(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max {
        return trimmed.to_string();
    }
    trimmed.chars().take(max).collect()
}

fn tool_calls_from(message: &Value) -> Vec<StoredToolCall> {
    if let Some(calls) = message.get("tool_calls").and_then(|value| value.as_array()) {
        return calls
            .iter()
            .filter_map(|call| {
                Some(StoredToolCall {
                    id: call
                        .get("id")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    name: call
                        .get("function")
                        .and_then(|value| value.get("name"))
                        .or_else(|| call.get("name"))
                        .and_then(|value| value.as_str())?
                        .to_string(),
                    arguments: match call
                        .get("function")
                        .and_then(|value| value.get("arguments"))
                        .or_else(|| call.get("arguments"))
                    {
                        Some(Value::String(raw)) => raw.clone(),
                        Some(other) => other.to_string(),
                        None => "{}".into(),
                    },
                })
            })
            .collect();
    }

    let Some(parts) = message.get("parts").and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    parts
        .iter()
        .filter(|part| part.get("type").and_then(|value| value.as_str()) == Some("tool-call"))
        .filter_map(|part| {
            Some(StoredToolCall {
                id: part
                    .get("toolCallId")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string(),
                name: part
                    .get("toolName")
                    .and_then(|value| value.as_str())?
                    .to_string(),
                arguments: match part.get("input") {
                    Some(Value::String(raw)) => raw.clone(),
                    Some(other) => other.to_string(),
                    None => "{}".into(),
                },
            })
        })
        .collect()
}

fn tool_call_id_from(message: &Value) -> Option<String> {
    if let Some(id) = message
        .get("tool_call_id")
        .or_else(|| message.get("toolCallId"))
        .and_then(|value| value.as_str())
    {
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    let parts = message.get("parts").and_then(|value| value.as_array())?;
    parts.iter().find_map(|part| {
        if part.get("type").and_then(|value| value.as_str()) == Some("tool-result") {
            part.get("toolCallId")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
        } else {
            None
        }
    })
}
