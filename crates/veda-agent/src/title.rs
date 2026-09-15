use std::sync::Arc;

use veda_llm::{Catalog, LlmClient};

pub const TITLE_INSTRUCTIONS: &str = "You craft brief chat titles from a single user request.\nReturn only the title text without quotes, markdown, numbering, or trailing punctuation.\nMaximum 8 words and 80 characters.\nMatch the language of the user request.";

pub fn is_placeholder_title(title: Option<&str>) -> bool {
    let title = title.unwrap_or("").trim();
    if title.is_empty() {
        return true;
    }
    if title == "__NEW_CHAT__" || title == "New Chat" {
        return true;
    }
    let prefixes = ["Новый чат", "New Chat"];
    for prefix in prefixes {
        if title == prefix {
            return true;
        }
        if let Some(rest) = title.strip_prefix(&format!("{prefix} ")) {
            if rest.chars().all(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

pub fn derive_provisional_title(user_request: &str) -> String {
    let text = user_request
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return String::new();
    }
    if text.chars().count() > 80 {
        return text.chars().take(80).collect::<String>().trim().to_string();
    }
    text
}

pub fn sanitize_generated_title(raw: &str) -> String {
    let mut title = raw.replace(['\r', '\n', '"', '\'', '`'], " ");
    title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    title = title.trim().to_string();
    if title.is_empty() {
        return String::new();
    }
    if title.starts_with('"') && title.ends_with('"') && title.len() >= 2 {
        title = title[1..title.len() - 1].trim().to_string();
    }
    title = title
        .trim_end_matches(['.', ',', ';', ':', '!', '?', ' '])
        .to_string();
    if title.chars().count() > 80 {
        title = title
            .chars()
            .take(80)
            .collect::<String>()
            .trim()
            .to_string();
    }
    title
}

pub async fn generate_title(
    llm: Arc<dyn LlmClient>,
    catalog: &Catalog,
    user_request: &str,
) -> String {
    let trimmed = user_request.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let Ok(model) = catalog.resolve(None) else {
        return derive_provisional_title(trimmed);
    };
    let prompt: String = trimmed.chars().take(2000).collect();
    let generated = llm
        .complete_text(model, TITLE_INSTRUCTIONS, &prompt)
        .await
        .ok()
        .map(|text| sanitize_generated_title(&text))
        .filter(|text| !text.is_empty());
    generated.unwrap_or_else(|| derive_provisional_title(trimmed))
}
