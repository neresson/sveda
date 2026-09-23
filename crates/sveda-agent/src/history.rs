use serde_json::{json, Value};
use sveda_llm::{ChatMessage, StoredToolCall};

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
        return (prompt, flatten_incoming(&stripped[..stripped.len() - 1]));
    }

    if last_role == "tool" {
        return (String::new(), flatten_incoming(&stripped));
    }

    (
        prompt.unwrap_or("").trim().to_string(),
        flatten_incoming(&stripped),
    )
}

fn flatten_incoming(messages: &[Value]) -> Vec<ChatMessage> {
    messages.iter().flat_map(to_chat_messages).collect()
}

pub fn strip_trailing_empty_assistant(messages: &[Value]) -> Vec<Value> {
    let mut items = messages.to_vec();
    while let Some(last) = items.last() {
        if last.get("role").and_then(|role| role.as_str()) != Some("assistant") {
            break;
        }
        if message_text(last).is_empty()
            && tool_calls_from(last).is_empty()
            && reasoning_from(last).is_empty()
        {
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

pub fn to_chat_messages(message: &Value) -> Vec<ChatMessage> {
    let role = message.get("role").and_then(Value::as_str).unwrap_or("");
    if role == "tool" {
        return to_chat_message(message).into_iter().collect();
    }
    let Some(mut chat) = to_chat_message(message) else {
        return Vec::new();
    };
    // iOS/web keep tool results as `tool-result` parts on the assistant message.
    // Providers need a separate `role: tool` item (function_call_output / tool_result).
    chat.tool_call_id = None;
    let mut out = vec![chat];
    out.extend(tool_result_messages(message));
    out
}

pub fn to_chat_message(message: &Value) -> Option<ChatMessage> {
    let role = message.get("role")?.as_str()?.to_string();
    Some(ChatMessage {
        role,
        content: message_text(message),
        reasoning: reasoning_from(message),
        tool_calls: tool_calls_from(message),
        tool_call_id: tool_call_id_from(message),
    })
}

fn tool_result_messages(message: &Value) -> Vec<ChatMessage> {
    let Some(parts) = message.get("parts").and_then(Value::as_array) else {
        return Vec::new();
    };
    parts
        .iter()
        .filter(|part| part.get("type").and_then(Value::as_str) == Some("tool-result"))
        .filter_map(|part| {
            let id = part
                .get("toolCallId")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())?;
            Some(ChatMessage {
                role: "tool".into(),
                content: part_output_text(part),
                tool_call_id: Some(id.to_string()),
                ..Default::default()
            })
        })
        .collect()
}

fn part_output_text(part: &Value) -> String {
    match part.get("output") {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => part
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    }
}

pub fn chat_to_json(message: &ChatMessage) -> Value {
    let mut value = json!({
        "role": message.role,
        "content": message.content,
    });
    if !message.reasoning.trim().is_empty() {
        value["reasoning"] = json!(message.reasoning);
    }
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
            reasoning: reasoning_from(message),
            tool_calls: tool_calls_from(message),
            tool_call_id: message
                .get("tool_call_id")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned),
        })
    })
}

#[allow(clippy::too_many_arguments)]
pub fn display_messages(
    incoming: &[Value],
    prompt: &str,
    assistant: &str,
    assistant_id: &str,
    reasoning: &str,
    tool_calls: &[StoredToolCall],
    tool_results: &[(String, String, String)],
    confirming_ids: &[String],
) -> Vec<Value> {
    let mut messages = strip_trailing_empty_assistant(incoming);
    let attached = attach_tool_results(&mut messages, tool_results);
    if messages.is_empty() && !prompt.trim().is_empty() {
        messages.push(json!({
            "id": "user_turn",
            "role": "user",
            "content": prompt,
            "parts": [{ "type": "text", "text": prompt }],
        }));
    }
    let mut parts = Vec::new();
    if !reasoning.trim().is_empty() {
        parts.push(json!({ "type": "reasoning", "text": reasoning }));
    }
    if !assistant.trim().is_empty() {
        parts.push(json!({ "type": "text", "text": assistant }));
    }
    for call in tool_calls {
        let input = serde_json::from_str(&call.arguments).unwrap_or(json!({}));
        let mut part = json!({
            "type": "tool-call",
            "toolCallId": call.id,
            "toolName": call.name,
            "target": "backend",
            "input": input,
        });
        if confirming_ids.iter().any(|id| id == &call.id) {
            part["confirmation"] = json!("required");
        }
        parts.push(part);
    }
    for (id, name, output) in tool_results {
        if attached.iter().any(|attached_id| attached_id == id) {
            continue;
        }
        let parsed = serde_json::from_str(output).unwrap_or(json!(output));
        parts.push(json!({
            "type": "tool-result",
            "toolCallId": id,
            "toolName": name,
            "output": parsed,
        }));
    }
    if !parts.is_empty() {
        let mut message = json!({
            "id": assistant_id,
            "role": "assistant",
            "content": assistant,
            "parts": parts,
        });
        if !reasoning.trim().is_empty() {
            message["reasoning"] = json!(reasoning);
        }
        messages.push(message);
    }
    messages
}

fn attach_tool_results(
    messages: &mut [Value],
    tool_results: &[(String, String, String)],
) -> Vec<String> {
    let mut attached = Vec::new();
    for (id, name, output) in tool_results {
        let already_resolved = messages.iter().any(|message| {
            message_parts(message).iter().any(|part| {
                part.get("type").and_then(Value::as_str) == Some("tool-result")
                    && part.get("toolCallId").and_then(Value::as_str) == Some(id)
            })
        });
        if already_resolved {
            attached.push(id.clone());
            continue;
        }
        let Some(message) = messages.iter_mut().find(|message| {
            message_parts(message).iter().any(|part| {
                part.get("type").and_then(Value::as_str) == Some("tool-call")
                    && part.get("toolCallId").and_then(Value::as_str) == Some(id)
            })
        }) else {
            continue;
        };
        let Some(parts) = message.get_mut("parts").and_then(Value::as_array_mut) else {
            continue;
        };
        let parsed = serde_json::from_str(output).unwrap_or(json!(output));
        parts.push(json!({
            "type": "tool-result",
            "toolCallId": id,
            "toolName": name,
            "output": parsed,
        }));
        attached.push(id.clone());
    }
    attached
}

fn message_parts(message: &Value) -> &[Value] {
    message
        .get("parts")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

pub fn pending_tool_calls(conversation: &[Value]) -> Vec<PendingToolCall> {
    let mut resolved = std::collections::HashSet::new();
    let mut calls = Vec::new();
    for message in conversation {
        let role = message.get("role").and_then(Value::as_str).unwrap_or("");
        if role == "tool" {
            if let Some(id) = message.get("tool_call_id").and_then(Value::as_str) {
                if !id.is_empty() {
                    resolved.insert(id.to_string());
                }
            }
            continue;
        }
        if role != "assistant" {
            continue;
        }
        let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) else {
            continue;
        };
        for call in tool_calls {
            let id = call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if id.is_empty() {
                continue;
            }
            let function = call.get("function");
            let name = function
                .and_then(|value| value.get("name"))
                .or_else(|| call.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let arguments = match function
                .and_then(|value| value.get("arguments"))
                .or_else(|| call.get("arguments"))
            {
                Some(Value::String(raw)) => serde_json::from_str(raw).unwrap_or(json!({})),
                Some(other) => other.clone(),
                None => json!({}),
            };
            calls.push(PendingToolCall {
                id,
                name,
                arguments,
            });
        }
    }
    calls
        .into_iter()
        .filter(|call| !resolved.contains(&call.id))
        .collect()
}

pub fn denied_tool_output() -> Value {
    json!({
        "success": false,
        "denied": true,
        "error": "The user denied this action and it was not executed."
    })
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

pub fn conversation_json(
    history: &[ChatMessage],
    prompt: &str,
    assistant: &str,
    reasoning: &str,
    tool_calls: &[StoredToolCall],
    tool_results: &[(String, String)],
) -> Vec<Value> {
    let mut items: Vec<Value> = history.iter().map(chat_to_json).collect();
    if !prompt.trim().is_empty() {
        items.push(json!({
            "role": "user",
            "content": prompt,
        }));
    }
    if !assistant.trim().is_empty() || !reasoning.trim().is_empty() || !tool_calls.is_empty() {
        items.push(chat_to_json(&ChatMessage {
            role: "assistant".into(),
            content: assistant.to_string(),
            reasoning: reasoning.to_string(),
            tool_calls: tool_calls.to_vec(),
            ..Default::default()
        }));
    }
    for (id, output) in tool_results {
        items.push(chat_to_json(&ChatMessage {
            role: "tool".into(),
            content: output.clone(),
            tool_call_id: Some(id.clone()),
            ..Default::default()
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

fn reasoning_from(message: &Value) -> String {
    if let Some(reasoning) = message.get("reasoning").and_then(|value| value.as_str()) {
        let trimmed = reasoning.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    let Some(parts) = message.get("parts").and_then(|value| value.as_array()) else {
        return String::new();
    };
    let texts: Vec<&str> = parts
        .iter()
        .filter(|part| {
            matches!(
                part.get("type").and_then(|value| value.as_str()),
                Some("reasoning") | Some("reasoning_text")
            )
        })
        .filter_map(|part| part.get("text").and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect();
    texts.join("\n")
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
        .filter(|id| !id.is_empty())
    {
        return Some(id.to_string());
    }
    if message.get("role").and_then(Value::as_str) != Some("tool") {
        return None;
    }
    let parts = message.get("parts").and_then(|value| value.as_array())?;
    parts.iter().find_map(|part| {
        if part.get("type").and_then(|value| value.as_str()) == Some("tool-result") {
            part.get("toolCallId")
                .and_then(|value| value.as_str())
                .filter(|id| !id.is_empty())
                .map(ToOwned::to_owned)
        } else {
            None
        }
    })
}
