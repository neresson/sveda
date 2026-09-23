use std::sync::Arc;

use sveda_llm::{Catalog, ChatMessage, LlmClient};

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
    let instructions = instructions
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let summary = summary.map(str::trim).filter(|value| !value.is_empty());
    match (instructions, summary) {
        (Some(existing), Some(summary)) => {
            Some(format!("{existing}\n\nConversation summary:\n{summary}"))
        }
        (Some(existing), None) => Some(existing),
        (None, Some(summary)) => Some(format!("Conversation summary:\n{summary}")),
        (None, None) => None,
    }
}

pub fn append_instruction(instructions: Option<String>, extra: &str) -> Option<String> {
    let extra = extra.trim();
    if extra.is_empty() {
        return instructions;
    }
    match instructions
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        Some(existing) => Some(format!("{existing}\n\n{extra}")),
        None => Some(extra.to_string()),
    }
}

pub fn host_tools_instruction(server_instructions: Option<&str>, tool_names: &[String]) -> String {
    let mut parts = Vec::new();
    if let Some(text) = server_instructions
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        parts.push(text.to_string());
    }
    if !tool_names.is_empty() {
        parts.push(format!(
            "You have host application tools: {}. When the user asks to create, update, search, or otherwise change host data, you MUST call the matching tool. Do not refuse by claiming you cannot publish, cannot act, or have no access. Do not only draft the result in chat.",
            tool_names.join(", ")
        ));
    }
    parts.join("\n\n")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_instruction_combines_existing_and_extra() {
        assert_eq!(
            append_instruction(Some("Base.".into()), "Extra."),
            Some("Base.\n\nExtra.".to_string())
        );
        assert_eq!(
            append_instruction(None, "Extra."),
            Some("Extra.".to_string())
        );
        assert_eq!(
            append_instruction(Some("Base.".into()), "  "),
            Some("Base.".to_string())
        );
        assert_eq!(append_instruction(None, ""), None);
    }

    #[test]
    fn host_tools_instruction_lists_tools_and_demands_usage() {
        let names = vec!["search_posts".to_string(), "create_post".to_string()];
        let text = host_tools_instruction(Some("Server rules."), &names);
        assert!(text.starts_with("Server rules."));
        assert!(text.contains("search_posts, create_post"));
        assert!(text.contains("MUST call the matching tool"));
    }

    #[test]
    fn host_tools_instruction_without_server_text_or_tools_is_empty() {
        assert_eq!(host_tools_instruction(None, &[]), "");
        let names = vec!["create_post".to_string()];
        let text = host_tools_instruction(None, &names);
        assert!(text.contains("create_post"));
        assert!(!text.starts_with("\n"));
    }
}
