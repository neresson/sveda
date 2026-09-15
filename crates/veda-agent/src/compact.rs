use std::sync::Arc;

use veda_llm::{Catalog, ChatMessage, LlmClient};

pub const COMPACT_INSTRUCTIONS: &str = "Summarize the conversation for continuation. Preserve goals, files or entities touched, recent tool commands and results, and the next step. Use the same language as the user. Do not call tools. Output plain text only.";

pub fn should_compact(count: usize, enabled: bool, min_messages: usize) -> bool {
    enabled && count >= min_messages.max(1)
}

pub fn tail_messages(messages: &[ChatMessage], keep_tail: usize) -> Vec<ChatMessage> {
    let keep = keep_tail.max(1);
    if messages.len() <= keep {
        return messages.to_vec();
    }
    messages[messages.len() - keep..].to_vec()
}

pub fn summary_source(messages: &[ChatMessage]) -> String {
    let mut lines = Vec::new();
    for message in messages {
        match message.role.as_str() {
            "user" => {
                if !message.content.trim().is_empty() {
                    lines.push(format!("User: {}", message.content.trim()));
                }
            }
            "assistant" if !message.tool_calls.is_empty() => {
                let names = message
                    .tool_calls
                    .iter()
                    .map(|call| call.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                if !names.is_empty() {
                    lines.push(format!("Assistant tools: {names}"));
                }
            }
            "tool" => {
                let clipped: String = message.content.chars().take(500).collect();
                lines.push(format!("Tool result: {clipped}"));
            }
            "assistant" => {
                let clipped: String = message.content.chars().take(400).collect();
                lines.push(format!("Assistant: {clipped}"));
            }
            _ => {}
        }
    }
    lines.join("\n")
}

pub fn inject_summary(instructions: Option<String>, summary: Option<&str>) -> Option<String> {
    let summary = summary.map(str::trim).filter(|value| !value.is_empty())?;
    let block = format!("Conversation summary:\n{summary}");
    match instructions
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        Some(existing) => Some(format!("{existing}\n\n{block}")),
        None => Some(block),
    }
}

pub async fn generate_summary(
    llm: Arc<dyn LlmClient>,
    catalog: &Catalog,
    messages: &[ChatMessage],
) -> String {
    let source = summary_source(messages);
    if source.is_empty() {
        return String::new();
    }
    let Ok(model) = catalog.resolve(None) else {
        return String::new();
    };
    llm.complete_text(model, COMPACT_INSTRUCTIONS, &source)
        .await
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_default()
}
