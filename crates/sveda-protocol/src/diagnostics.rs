/// Only use this for fields whose wire contract is an identifier, never for
/// free-form error text, URLs, or arbitrary headers.
pub fn identifier(value: &str) -> &str {
    if value.is_empty() {
        return "";
    }
    if value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        && !["sveda_embed_", "veda_embed_", "sk-"]
            .iter()
            .any(|prefix| value.starts_with(prefix))
    {
        value
    } else {
        "[redacted]"
    }
}
use std::sync::OnceLock;

use serde_json::{json, Value};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Off,
    Warn,
    Info,
    Debug,
}

#[derive(Clone, Default)]
pub struct Context {
    pub chat_id: String,
    pub turn_id: String,
    pub message_id: String,
    pub step: Option<u32>,
}

tokio::task_local! {
    pub static CONTEXT: Context;
}

impl Context {
    pub fn current() -> Self {
        CONTEXT.try_with(Clone::clone).unwrap_or_default()
    }
}

pub fn enabled(level: Level) -> bool {
    static LEVEL: OnceLock<Level> = OnceLock::new();
    level
        <= *LEVEL.get_or_init(|| {
            match std::env::var("SVEDA_LOG_LEVEL")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str()
            {
                "off" => Level::Off,
                "warn" => Level::Warn,
                "debug" => Level::Debug,
                _ => Level::Info,
            }
        })
}

pub fn log(level: Level, event: &'static str, fields: impl FnOnce() -> Value) {
    if !enabled(level) {
        return;
    }
    let context = Context::current();
    let level = match level {
        Level::Off => return,
        Level::Warn => "WARN",
        Level::Info => "INFO",
        Level::Debug => "DEBUG",
    };
    eprintln!(
        "{}",
        json!({
            "timestamp": chrono::Utc::now(),
            "level": level,
            "event": event,
            "chat_id": chat_id(&context.chat_id),
            "turn_id": context.turn_id,
            "message_id": context.message_id,
            "step": context.step,
            "fields": fields(),
        })
    );
}

pub fn chat_id(value: &str) -> &str {
    if value.is_empty() || value.starts_with("chat_") {
        identifier(value)
    } else {
        "[redacted]"
    }
}
