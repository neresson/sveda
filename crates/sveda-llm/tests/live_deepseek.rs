use std::env;
use std::fs;
use std::path::PathBuf;

use futures_util::StreamExt;
use sveda_llm::{Catalog, ChatMessage, HttpClient, LlmChunk, LlmClient, StepRequest};

fn live_key() -> Option<String> {
    for name in ["DEEPSEEK_API_KEY", "SVEDA_DEEPSEEK_API_KEY"] {
        if let Ok(value) = env::var(name) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "../../apps/runtime/.env",
        "../../../sveda/apps/runtime/.env",
    ] {
        let Ok(contents) = fs::read_to_string(manifest.join(relative)) else {
            continue;
        };
        for raw in contents.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let line = line.strip_prefix("export ").unwrap_or(line).trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if key.trim() != "DEEPSEEK_API_KEY" && key.trim() != "SVEDA_DEEPSEEK_API_KEY" {
                continue;
            }
            let mut value = value.trim().to_string();
            if (value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\''))
            {
                value = value[1..value.len() - 1].to_string();
            }
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn require_live_llm() -> bool {
    matches!(
        env::var("SVEDA_REQUIRE_LIVE_LLM")
            .ok()
            .as_deref()
            .map(str::trim),
        Some("1") | Some("true") | Some("yes")
    )
}

fn request(thinking: bool, prompt: &str) -> StepRequest {
    StepRequest {
        messages: vec![ChatMessage {
            role: "user".into(),
            content: prompt.into(),
            ..Default::default()
        }],
        instructions: Some("Reply in one short sentence.".into()),
        thinking,
        tools: Vec::new(),
    }
}

async fn collect(thinking: bool, prompt: &str) -> Result<(String, String), String> {
    let key = live_key().expect("DEEPSEEK_API_KEY");
    let catalog = Catalog::builtin(key);
    let model = catalog.resolve(None).expect("default model");
    let llm = HttpClient::from_timeout(90);
    let mut stream = llm.stream_step(model, request(thinking, prompt));
    let mut text = String::new();
    let mut reasoning = String::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(LlmChunk::TextDelta(delta)) => text.push_str(&delta),
            Ok(LlmChunk::ReasoningDelta(delta)) => reasoning.push_str(&delta),
            Ok(LlmChunk::End { .. }) => break,
            Ok(_) => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok((text, reasoning))
}

#[tokio::test]
async fn live_deepseek_answers_without_thinking() {
    if live_key().is_none() {
        eprintln!("skip live DeepSeek: DEEPSEEK_API_KEY is not set");
        return;
    }
    let (text, _) = match collect(false, "Reply with exactly the word PONG and nothing else.").await
    {
        Ok(value) => value,
        Err(error) if require_live_llm() => panic!("live DeepSeek failed: {error}"),
        Err(error) => {
            eprintln!("skip live DeepSeek: {error}");
            return;
        }
    };
    assert!(
        text.to_uppercase().contains("PONG"),
        "unexpected DeepSeek reply: {text:?}"
    );
}

#[tokio::test]
async fn live_deepseek_can_think_before_answering() {
    if live_key().is_none() {
        eprintln!("skip live DeepSeek: DEEPSEEK_API_KEY is not set");
        return;
    }
    let (text, reasoning) = match collect(true, "What is 12 + 7? Reply with the number only.").await
    {
        Ok(value) => value,
        Err(error) if require_live_llm() => panic!("live DeepSeek failed: {error}"),
        Err(error) => {
            eprintln!("skip live DeepSeek: {error}");
            return;
        }
    };
    assert!(
        text.contains('1') && text.contains('9'),
        "unexpected DeepSeek reply: {text:?}"
    );
    let _ = reasoning;
}
